//! Sandboxed pre-request/test script engine using `boa_engine` (pure-Rust JS interpreter).
//!
//! Clean-room scripting surface under `bapu.*` (with `pm.*` and `postman.*` aliases for
//! seamless migration of existing test collections):
//! - `bapu.environment` / `bapu.globals` / `bapu.collectionVariables` / `bapu.variables`
//! - `bapu.cookies.get(name)` / `bapu.cookies.has(name)` / `bapu.cookies.toObject()`
//! - `bapu.execution.setNextRequest(nameOrNull)` / `postman.setNextRequest(nameOrNull)`
//! - `bapu.info` (`requestName`, `iteration`)
//! - `bapu.test(name, fn)` & `bapu.expect(actual)` (`toBe`, `toEqual`, `toMatch`, `toContain`,
//!   `to.have.status`, `to.have.header`, `to.have.jsonBody`, `to.be.ok`, `to.be.json`, ...)

use crate::types::{ApiRequest, ApiResponse, Environment, KeyValuePair, TestAssertionResult};
use boa_engine::{
    js_string, native_function::NativeFunction, object::ObjectInitializer, property::Attribute,
    Context, JsResult, JsValue, Source,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct ScriptExecutionResult {
    pub test_results: Vec<TestAssertionResult>,
    pub logs: Vec<String>,
    pub updated_env_vars: HashMap<String, String>,
    pub updated_globals: HashMap<String, String>,
    pub updated_collection_vars: HashMap<String, String>,
    /// `None` => sequential; `Some(Some(name))` => jump to request `name`; `Some(None)` => stop run.
    pub next_request: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptExtraContext {
    pub globals: Vec<KeyValuePair>,
    pub collection_vars: Vec<KeyValuePair>,
    pub iteration: usize,
}

const SCRIPT_TIMEOUT: Duration = Duration::from_secs(5);

const EXPECT_HARNESS_JS: &str = r#"
function expect(actual) {
  return {
    toBe: function (expected) {
      if (actual !== expected) throw new Error('Expected ' + JSON.stringify(expected) + ' but got ' + JSON.stringify(actual));
    },
    toEqual: function (expected) {
      if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error('Expected ' + JSON.stringify(expected) + ' but got ' + JSON.stringify(actual));
    },
    toBeLessThan: function (max) {
      if (typeof actual !== 'number' || actual >= max) throw new Error('Expected ' + actual + ' to be less than ' + max);
    },
    toBeGreaterThan: function (min) {
      if (typeof actual !== 'number' || actual <= min) throw new Error('Expected ' + actual + ' to be greater than ' + min);
    },
    toContain: function (item) {
      if (typeof actual === 'string' || Array.isArray(actual)) {
        if (actual.indexOf(item) === -1) throw new Error('Expected ' + JSON.stringify(actual) + ' to contain ' + JSON.stringify(item));
      } else {
        throw new Error('Cannot check toContain on ' + typeof actual);
      }
    },
    toMatch: function (pattern) {
      var rx = (pattern instanceof RegExp) ? pattern : new RegExp(String(pattern));
      if (!rx.test(String(actual))) throw new Error('Expected "' + actual + '" to match ' + rx);
    },
    toBeDefined: function () {
      if (actual === undefined || actual === null) throw new Error('Expected value to be defined, got ' + actual);
    },
    toBeTruthy: function () {
      if (!actual) throw new Error('Expected truthy value, got ' + actual);
    },
    toBeFalsy: function () {
      if (actual) throw new Error('Expected falsy value, got ' + actual);
    },
    to: {
      equal: function (expected) {
        if (actual !== expected) throw new Error('Expected ' + JSON.stringify(expected) + ' but got ' + JSON.stringify(actual));
      },
      eql: function (expected) {
        if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error('Expected ' + JSON.stringify(expected) + ' but got ' + JSON.stringify(actual));
      },
      be: {
        below: function (max) { if (typeof actual !== 'number' || actual >= max) throw new Error('Expected ' + actual + ' to be below ' + max); },
        above: function (min) { if (typeof actual !== 'number' || actual <= min) throw new Error('Expected ' + actual + ' to be above ' + min); },
        oneOf: function (list) { if (list.indexOf(actual) === -1) throw new Error('Expected ' + JSON.stringify(actual) + ' to be one of ' + JSON.stringify(list)); },
        true: function () { if (actual !== true) throw new Error('Expected true, got ' + actual); },
        false: function () { if (actual !== false) throw new Error('Expected false, got ' + actual); },
        null: function () { if (actual !== null) throw new Error('Expected null, got ' + actual); },
        ok: function () { if (!actual) throw new Error('Expected truthy value, got ' + actual); }
      },
      have: {
        property: function (propName, expectedVal) {
          if (!actual || !(propName in actual)) throw new Error('Expected object to have property "' + propName + '"');
          if (expectedVal !== undefined && actual[propName] !== expectedVal) throw new Error('Expected property "' + propName + '" to equal ' + expectedVal + ', got ' + actual[propName]);
        },
        status: function (code) {
          var status = actual && actual.status !== undefined ? actual.status : (actual && actual.code !== undefined ? actual.code : actual);
          if (status !== code) throw new Error('Expected status ' + code + ', got ' + status);
        },
        lengthOf: function (len) {
          if (!actual || actual.length !== len) throw new Error('Expected length of ' + len + ', got ' + (actual && actual.length));
        }
      },
      include: function (item) {
        if (typeof actual === 'string' || Array.isArray(actual)) {
          if (actual.indexOf(item) === -1) throw new Error('Expected ' + JSON.stringify(actual) + ' to include ' + JSON.stringify(item));
        }
      }
    }
  };
}
bapu.expect = expect;
pm.expect = expect;
bapu.test = test;
pm.test = test;
bapu.log = console.log;
pm.log = console.log;
bapu.execution = { setNextRequest: __setNextRequest };
pm.execution = { setNextRequest: __setNextRequest };
var postman = { setNextRequest: __setNextRequest };

if (typeof response !== 'undefined') {
  response.code = response.status;
  response.responseTime = response.timeMs;
  response.json = function () { return response.data; };
  response.text = function () { return typeof response.data === 'string' ? response.data : JSON.stringify(response.data); };
  response.to = {
    have: {
      status: function (code) {
        if (response.status !== code) throw new Error('Expected status ' + code + ', got ' + response.status);
      },
      header: function (key, expectedVal) {
        var lk = String(key).toLowerCase();
        var hdrs = response.headers || {};
        var found = Object.keys(hdrs).find(function (k) { return k.toLowerCase() === lk; });
        if (!found) throw new Error('Expected header "' + key + '" to exist');
        if (expectedVal !== undefined && hdrs[found] !== expectedVal) {
          throw new Error('Expected header "' + key + '" to equal "' + expectedVal + '", got "' + hdrs[found] + '"');
        }
      },
      jsonBody: function (path) {
        if (!path) {
          if (typeof response.data !== 'object' || response.data === null) throw new Error('Expected JSON object/array body');
          return;
        }
        var cur = response.data;
        var parts = String(path).split('.');
        for (var i = 0; i < parts.length; i++) {
          if (cur === null || cur === undefined || !(parts[i] in Object(cur))) {
            throw new Error('Expected JSON body to contain path "' + path + '"');
          }
          cur = cur[parts[i]];
        }
      }
    },
    be: {
      ok: function () {
        if (response.status < 200 || response.status >= 300) throw new Error('Expected 2xx status, got ' + response.status);
      },
      json: function () {
        var hdrs = response.headers || {};
        var ctKey = Object.keys(hdrs).find(function (k) { return k.toLowerCase() === 'content-type'; });
        var ct = ctKey ? hdrs[ctKey] : '';
        if (ct.indexOf('json') === -1 && typeof response.data !== 'object') {
          throw new Error('Expected JSON response');
        }
      }
    }
  };
  bapu.response = response;
  pm.response = response;

  var __cookieMap = {};
  (response.cookies || []).forEach(function (c) { __cookieMap[c.name] = c.value; });
  var cookiesApi = {
    has: function (n) { return Object.prototype.hasOwnProperty.call(__cookieMap, n); },
    get: function (n) { return __cookieMap[n]; },
    toObject: function () { return Object.assign({}, __cookieMap); }
  };
  bapu.cookies = cookiesApi;
  pm.cookies = cookiesApi;
}
"#;

struct SharedState {
    env_vars: HashMap<String, String>,
    updated_env: HashMap<String, String>,
    globals: HashMap<String, String>,
    updated_globals: HashMap<String, String>,
    col_vars: HashMap<String, String>,
    updated_col: HashMap<String, String>,
    logs: Vec<String>,
    test_results: Vec<TestAssertionResult>,
    next_request: Option<Option<String>>,
}

pub fn execute_pre_request_script(
    script: &str,
    request: &ApiRequest,
    env: &Environment,
) -> ScriptExecutionResult {
    execute_script_with_context(script, request, env, None, &ScriptExtraContext::default())
}

pub fn execute_test_script(
    script: &str,
    response: &ApiResponse,
    request: &ApiRequest,
    env: &Environment,
) -> ScriptExecutionResult {
    execute_script_with_context(
        script,
        request,
        env,
        Some(response),
        &ScriptExtraContext::default(),
    )
}

pub fn execute_script_with_context(
    script: &str,
    request: &ApiRequest,
    env: &Environment,
    response: Option<&ApiResponse>,
    extra: &ScriptExtraContext,
) -> ScriptExecutionResult {
    if script.trim().is_empty() {
        return ScriptExecutionResult::default();
    }

    let script_owned = script.to_string();
    let req_name = request.name.clone();
    let env_owned = env.clone();
    let resp_owned = response.cloned();
    let extra_owned = extra.clone();
    let is_test = resp_owned.is_some();

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let res = run_sync(&script_owned, &req_name, &env_owned, resp_owned.as_ref(), &extra_owned);
        let _ = tx.send(res);
    });

    match rx.recv_timeout(SCRIPT_TIMEOUT) {
        Ok(res) => res,
        Err(_) => {
            let mut res = ScriptExecutionResult::default();
            let msg = format!("Script timed out after {}ms", SCRIPT_TIMEOUT.as_millis());
            if is_test {
                res.test_results.push(TestAssertionResult {
                    name: "Script Timeout".into(),
                    passed: false,
                    error: Some(msg.clone()),
                });
            }
            res.logs.push(format!("[Script Error]: {msg}"));
            res
        }
    }
}

fn run_sync(
    script: &str,
    req_name: &str,
    env: &Environment,
    response: Option<&ApiResponse>,
    extra: &ScriptExtraContext,
) -> ScriptExecutionResult {
    let to_map = |kvs: &[KeyValuePair]| -> HashMap<String, String> {
        kvs.iter()
            .filter(|kv| kv.enabled)
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect()
    };

    let state = Rc::new(RefCell::new(SharedState {
        env_vars: to_map(&env.variables),
        updated_env: HashMap::new(),
        globals: to_map(&extra.globals),
        updated_globals: HashMap::new(),
        col_vars: to_map(&extra.collection_vars),
        updated_col: HashMap::new(),
        logs: Vec::new(),
        test_results: Vec::new(),
        next_request: None,
    }));

    let mut context = Context::default();
    if let Err(e) = bind_globals(&mut context, &state, req_name, extra.iteration, response) {
        state.borrow_mut().logs.push(format!("[Script Setup Error]: {e}"));
        return finish(state);
    }

    if let Err(e) = context.eval(Source::from_bytes(EXPECT_HARNESS_JS)) {
        state.borrow_mut().logs.push(format!("[Harness Error]: {e}"));
        return finish(state);
    }

    if let Err(e) = context.eval(Source::from_bytes(script)) {
        let message = format!("{e}");
        let mut s = state.borrow_mut();
        if response.is_some() {
            s.test_results.push(TestAssertionResult {
                name: "Script Syntax / Execution".into(),
                passed: false,
                error: Some(format!("Syntax Error: {message}")),
            });
        } else {
            s.logs.push(format!("[Script Error]: {message}"));
        }
    }

    finish(state)
}

fn finish(state: Rc<RefCell<SharedState>>) -> ScriptExecutionResult {
    let s = state.borrow();
    ScriptExecutionResult {
        test_results: s.test_results.clone(),
        logs: s.logs.clone(),
        updated_env_vars: s.updated_env.clone(),
        updated_globals: s.updated_globals.clone(),
        updated_collection_vars: s.updated_col.clone(),
        next_request: s.next_request.clone(),
    }
}

#[derive(Clone, Copy)]
enum ScopeKind {
    Env,
    Globals,
    Collection,
}

fn build_scope_object(
    context: &mut Context,
    state: &Rc<RefCell<SharedState>>,
    scope: ScopeKind,
) -> boa_engine::JsObject {
    let get_s = state.clone();
    let set_s = state.clone();
    let has_s = state.clone();
    let unset_s = state.clone();

    // SAFETY: Closures capture only `Rc<RefCell<SharedState>>` + `ScopeKind` (Copy), zero GC handles.
    let get_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key = arg_as_string(args, 0, ctx)?;
            let s = get_s.borrow();
            let val = match scope {
                ScopeKind::Env => s.env_vars.get(&key),
                ScopeKind::Globals => s.globals.get(&key),
                ScopeKind::Collection => s.col_vars.get(&key),
            }
            .cloned()
            .unwrap_or_default();
            Ok(JsValue::from(js_string!(val)))
        })
    };
    let set_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key = arg_as_string(args, 0, ctx)?;
            let val = arg_as_string(args, 1, ctx)?;
            let mut s = set_s.borrow_mut();
            match scope {
                ScopeKind::Env => {
                    s.env_vars.insert(key.clone(), val.clone());
                    s.updated_env.insert(key, val);
                }
                ScopeKind::Globals => {
                    s.globals.insert(key.clone(), val.clone());
                    s.updated_globals.insert(key, val);
                }
                ScopeKind::Collection => {
                    s.col_vars.insert(key.clone(), val.clone());
                    s.updated_col.insert(key, val);
                }
            }
            Ok(JsValue::undefined())
        })
    };
    let has_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key = arg_as_string(args, 0, ctx)?;
            let s = has_s.borrow();
            let present = match scope {
                ScopeKind::Env => s.env_vars.contains_key(&key),
                ScopeKind::Globals => s.globals.contains_key(&key),
                ScopeKind::Collection => s.col_vars.contains_key(&key),
            };
            Ok(JsValue::from(present))
        })
    };
    let unset_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key = arg_as_string(args, 0, ctx)?;
            let mut s = unset_s.borrow_mut();
            match scope {
                ScopeKind::Env => {
                    s.env_vars.remove(&key);
                    s.updated_env.remove(&key);
                }
                ScopeKind::Globals => {
                    s.globals.remove(&key);
                    s.updated_globals.remove(&key);
                }
                ScopeKind::Collection => {
                    s.col_vars.remove(&key);
                    s.updated_col.remove(&key);
                }
            }
            Ok(JsValue::undefined())
        })
    };

    ObjectInitializer::new(context)
        .function(get_fn, js_string!("get"), 1)
        .function(set_fn, js_string!("set"), 2)
        .function(has_fn, js_string!("has"), 1)
        .function(unset_fn, js_string!("unset"), 1)
        .build()
}

fn bind_globals(
    context: &mut Context,
    state: &Rc<RefCell<SharedState>>,
    req_name: &str,
    iteration: usize,
    response: Option<&ApiResponse>,
) -> JsResult<()> {
    // SAFETY: Each closure captures only pure Rust data (`Rc<RefCell<SharedState>>`), zero Boa GC handles.
    let log_state = state.clone();
    let log_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let line = args
                .iter()
                .map(|a| a.to_string(ctx).map(|s| s.to_std_string_escaped()).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(" ");
            log_state.borrow_mut().logs.push(line);
            Ok(JsValue::undefined())
        })
    };
    let console = ObjectInitializer::new(context)
        .function(log_fn.clone(), js_string!("log"), 0)
        .function(log_fn.clone(), js_string!("info"), 0)
        .function(log_fn.clone(), js_string!("warn"), 0)
        .function(log_fn, js_string!("error"), 0)
        .build();
    context.register_global_property(js_string!("console"), console, Attribute::all())?;

    let env_obj = build_scope_object(context, state, ScopeKind::Env);
    let globals_obj = build_scope_object(context, state, ScopeKind::Globals);
    let col_vars_obj = build_scope_object(context, state, ScopeKind::Collection);
    context.register_global_property(js_string!("env"), env_obj.clone(), Attribute::all())?;

    let info_obj = ObjectInitializer::new(context)
        .property(
            js_string!("requestName"),
            JsValue::from(js_string!(req_name)),
            Attribute::all(),
        )
        .property(
            js_string!("iteration"),
            JsValue::from(iteration as i32),
            Attribute::all(),
        )
        .build();

    let bapu_obj = ObjectInitializer::new(context)
        .property(js_string!("environment"), env_obj.clone(), Attribute::all())
        .property(js_string!("variables"), env_obj.clone(), Attribute::all())
        .property(js_string!("env"), env_obj, Attribute::all())
        .property(js_string!("globals"), globals_obj, Attribute::all())
        .property(js_string!("collectionVariables"), col_vars_obj, Attribute::all())
        .property(js_string!("info"), info_obj, Attribute::all())
        .build();
    context.register_global_property(js_string!("bapu"), bapu_obj.clone(), Attribute::all())?;
    context.register_global_property(js_string!("pm"), bapu_obj, Attribute::all())?;

    let next_state = state.clone();
    let set_next_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let first = args.first().cloned().unwrap_or(JsValue::null());
            if first.is_null_or_undefined() {
                next_state.borrow_mut().next_request = Some(None);
            } else {
                let target = first.to_string(ctx)?.to_std_string_escaped();
                next_state.borrow_mut().next_request = Some(Some(target));
            }
            Ok(JsValue::undefined())
        })
    };
    context.register_global_callable(js_string!("__setNextRequest"), 1, set_next_fn)?;

    if let Some(resp) = response {
        let response_json = serde_json::to_string(resp).unwrap_or_default();
        let response_js = context
            .eval(Source::from_bytes(&format!("({response_json})")))
            .unwrap_or(JsValue::undefined());
        context.register_global_property(
            js_string!("response"),
            response_js.clone(),
            Attribute::all(),
        )?;
        context.register_global_property(js_string!("res"), response_js, Attribute::all())?;
    }

    let test_state = state.clone();
    let test_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let name = arg_as_string(args, 0, ctx)?;
            let callback = args.get(1).cloned().unwrap_or_default();
            let passed_result = callback
                .as_callable()
                .map(|c| c.call(&JsValue::undefined(), &[], ctx));
            let mut s = test_state.borrow_mut();
            match passed_result {
                Some(Ok(_)) => s.test_results.push(TestAssertionResult {
                    name,
                    passed: true,
                    error: None,
                }),
                Some(Err(e)) => s.test_results.push(TestAssertionResult {
                    name,
                    passed: false,
                    error: Some(format!("{e}")),
                }),
                None => s.test_results.push(TestAssertionResult {
                    name,
                    passed: false,
                    error: Some("second argument to test() was not a function".into()),
                }),
            }
            Ok(JsValue::undefined())
        })
    };
    context.register_global_callable(js_string!("test"), 2, test_fn)?;

    Ok(())
}

fn arg_as_string(args: &[JsValue], index: usize, ctx: &mut Context) -> JsResult<String> {
    Ok(args
        .get(index)
        .cloned()
        .unwrap_or(JsValue::undefined())
        .to_string(ctx)?
        .to_std_string_escaped())
}

pub fn request_has_executable_script(req: &ApiRequest) -> bool {
    req.has_executable_script()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CookieEntry;

    fn dummy_req() -> ApiRequest {
        ApiRequest {
            name: "Get Users".into(),
            url: "https://example.com".into(),
            ..Default::default()
        }
    }

    fn dummy_env() -> Environment {
        Environment {
            id: "e1".into(),
            name: "Dev".into(),
            variables: vec![KeyValuePair {
                id: "v1".into(),
                key: "API_HOST".into(),
                value: "https://api.test".into(),
                enabled: true,
                is_secret: Some(false),
            }],
        }
    }

    #[test]
    fn pre_request_reads_and_updates_env_globals_and_next_request() {
        let res = execute_pre_request_script(
            r#"
            const host = pm.environment.get("API_HOST");
            console.log("host:", host);
            pm.environment.set("DERIVED", host + "/v2");
            bapu.globals.set("GLOBAL_TOKEN", "g_123");
            bapu.collectionVariables.set("COL_ID", "c_99");
            bapu.execution.setNextRequest("Verify User");
            "#,
            &dummy_req(),
            &dummy_env(),
        );
        assert_eq!(
            res.updated_env_vars.get("DERIVED").map(String::as_str),
            Some("https://api.test/v2")
        );
        assert_eq!(
            res.updated_globals.get("GLOBAL_TOKEN").map(String::as_str),
            Some("g_123")
        );
        assert_eq!(
            res.updated_collection_vars.get("COL_ID").map(String::as_str),
            Some("c_99")
        );
        assert_eq!(res.next_request, Some(Some("Verify User".into())));
    }

    #[test]
    fn test_script_runs_rich_response_and_cookie_assertions() {
        let mut headers = HashMap::new();
        headers.insert("content-type".into(), "application/json".into());
        let resp = ApiResponse {
            status: 200,
            status_text: "OK".into(),
            time_ms: 42,
            size_bytes: 15,
            headers,
            cookies: vec![CookieEntry {
                domain: "example.com".into(),
                path: "/".into(),
                name: "session_id".into(),
                value: "xyz987".into(),
                secure: true,
                http_only: true,
                expires: None,
            }],
            data: serde_json::json!({ "ok": true, "items": [1, 2, 3], "email": "ada@example.com" }),
            timestamp: "2026-01-01T00:00:00Z".into(),
            http_version: None,
            remote_addr: None,
            test_results: vec![],
            console_logs: vec![],
        };
        let res = execute_test_script(
            r#"
            pm.test("status 200 & ok & json", () => {
                pm.response.to.have.status(200);
                pm.response.to.be.ok();
                pm.response.to.be.json();
                pm.response.to.have.jsonBody("email");
            });
            pm.test("cookie & regex match", () => {
                expect(pm.cookies.get("session_id")).toBe("xyz987");
                expect(response.json().email).toMatch(/^ada@/);
                expect(response.json().items).to.have.lengthOf(3);
            });
            pm.test("intentional failure", () => {
                expect(response.status).toBe(404);
            });
            "#,
            &resp,
            &dummy_req(),
            &dummy_env(),
        );
        assert_eq!(res.test_results.len(), 3);
        assert!(res.test_results[0].passed);
        assert!(res.test_results[1].passed);
        assert!(!res.test_results[2].passed);
    }

    #[test]
    fn sandbox_has_no_fetch_or_process() {
        let res = execute_pre_request_script(
            r#"
            pm.environment.set("HAS_FETCH", String(typeof fetch));
            pm.environment.set("HAS_PROCESS", String(typeof process));
            "#,
            &dummy_req(),
            &dummy_env(),
        );
        assert_eq!(res.updated_env_vars.get("HAS_FETCH").map(String::as_str), Some("undefined"));
        assert_eq!(res.updated_env_vars.get("HAS_PROCESS").map(String::as_str), Some("undefined"));
    }
}
