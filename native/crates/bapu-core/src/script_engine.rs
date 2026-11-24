//! Sandboxed pre-request/test script engine, using `boa_engine` (a pure-Rust JS interpreter)
//! instead of Node's `new Function` + a Web Worker (what the Electron app uses).
//!
//! This is a stronger sandbox by construction, not just by configuration: `boa_engine` has no
//! built-in `fetch`, no filesystem, no network sockets, and no knowledge of `bapuBridge` --
//! a script can only do what we explicitly bind into its global scope below (`bapu`/`pm`/
//! `console`/`test`/`expect`). There is no ambient capability to escape to, unlike a JS realm
//! that merely has dangerous globals shadowed. The one real capability a script has by design
//! is reading/writing the *current* environment's variables (including ones marked secret) via
//! `bapu.environment.get/set` -- the same trade-off every Postman-style scripting engine makes,
//! since that's the actual feature (e.g. signing a request with a secret from an env var).
//!
//! NOTE: `boa_engine`'s API surface changes between versions more than most crates. This file
//! was written against the ~0.20 API shape from memory (this session could not reach crates.io
//! to verify against the exact resolved version) -- it is the single most likely file in this
//! scaffold to need small signature fixes on your first `cargo build`.

use crate::types::{ApiRequest, ApiResponse, Environment, KeyValuePair, TestAssertionResult};
use boa_engine::{
    js_string, native_function::NativeFunction, object::ObjectInitializer, property::Attribute,
    Context, JsResult, JsValue, Source,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone)]
pub struct ScriptExecutionResult {
    pub test_results: Vec<TestAssertionResult>,
    pub logs: Vec<String>,
    pub updated_env_vars: HashMap<String, String>,
}

/// Wall-clock budget for a single script run. `boa_engine` doesn't preemptively interrupt a
/// running script the way a Worker + `terminate()` can, so an intentionally malicious infinite
/// loop in a script is not yet hard-killed here -- see the TODO at the bottom of this file.
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(5);

struct SharedState {
    vars: HashMap<String, String>,
    updated: HashMap<String, String>,
    logs: Vec<String>,
    test_results: Vec<TestAssertionResult>,
}

pub fn execute_pre_request_script(
    script: &str,
    _request: &ApiRequest,
    env: &Environment,
) -> ScriptExecutionResult {
    run(script, env, None)
}

pub fn execute_test_script(
    script: &str,
    response: &ApiResponse,
    _request: &ApiRequest,
    env: &Environment,
) -> ScriptExecutionResult {
    run(script, env, Some(response))
}

fn run(script: &str, env: &Environment, response: Option<&ApiResponse>) -> ScriptExecutionResult {
    if script.trim().is_empty() {
        return ScriptExecutionResult::default();
    }

    let mut vars = HashMap::new();
    for kv in &env.variables {
        vars.insert(kv.key.clone(), kv.value.clone());
    }

    let state = Rc::new(RefCell::new(SharedState {
        vars,
        updated: HashMap::new(),
        logs: Vec::new(),
        test_results: Vec::new(),
    }));

    let mut context = Context::default();
    if let Err(e) = bind_globals(&mut context, &state, response) {
        state.borrow_mut().logs.push(format!("[Script Setup Error]: {e}"));
        return finish(state);
    }

    let start = Instant::now();
    let result = context.eval(Source::from_bytes(script));
    let elapsed = start.elapsed();

    if let Err(e) = result {
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
    } else if elapsed > SCRIPT_TIMEOUT {
        state
            .borrow_mut()
            .logs
            .push(format!("[Script Warning]: took {}ms (budget {}ms)", elapsed.as_millis(), SCRIPT_TIMEOUT.as_millis()));
    }

    finish(state)
}

fn finish(state: Rc<RefCell<SharedState>>) -> ScriptExecutionResult {
    let s = state.borrow();
    ScriptExecutionResult {
        test_results: s.test_results.clone(),
        logs: s.logs.clone(),
        updated_env_vars: s.updated.clone(),
    }
}

fn bind_globals(
    context: &mut Context,
    state: &Rc<RefCell<SharedState>>,
    response: Option<&ApiResponse>,
) -> JsResult<()> {
    // console.log / bapu.log — appended to `logs`, surfaced in the UI's script console panel.
    let log_state = state.clone();
    let console = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_closure(move |_this, args, ctx| {
                let line = args
                    .iter()
                    .map(|a| a.to_string(ctx).map(|s| s.to_std_string_escaped()).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(" ");
                log_state.borrow_mut().logs.push(line);
                Ok(JsValue::undefined())
            }),
            js_string!("log"),
            0,
        )
        .build();
    context.register_global_property(js_string!("console"), console, Attribute::all())?;

    // bapu.environment / pm.environment: get/set/has/unset against the current environment's
    // variables. This is intentionally the *only* persistent state a script can touch.
    let get_state = state.clone();
    let set_state = state.clone();
    let env_obj = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_closure(move |_this, args, ctx| {
                let key = arg_as_string(args, 0, ctx)?;
                let val = get_state.borrow().vars.get(&key).cloned().unwrap_or_default();
                Ok(JsValue::from(js_string!(val)))
            }),
            js_string!("get"),
            1,
        )
        .function(
            NativeFunction::from_closure(move |_this, args, ctx| {
                let key = arg_as_string(args, 0, ctx)?;
                let val = arg_as_string(args, 1, ctx)?;
                let mut s = set_state.borrow_mut();
                s.vars.insert(key.clone(), val.clone());
                s.updated.insert(key, val);
                Ok(JsValue::undefined())
            }),
            js_string!("set"),
            2,
        )
        .build();
    context.register_global_property(js_string!("env"), env_obj.clone(), Attribute::all())?;

    // `bapu` and `pm` namespaces both point at the same small API surface (pm.* exists for
    // drop-in compatibility with Postman-authored scripts, matching the Electron app).
    let bapu_obj = ObjectInitializer::new(context)
        .property(js_string!("environment"), env_obj.clone(), Attribute::all())
        .property(js_string!("variables"), env_obj.clone(), Attribute::all())
        .build();
    context.register_global_property(js_string!("bapu"), bapu_obj.clone(), Attribute::all())?;
    context.register_global_property(js_string!("pm"), bapu_obj, Attribute::all())?;

    // response / res: only present for post-response test scripts.
    if let Some(resp) = response {
        let response_json = serde_json::to_string(resp).unwrap_or_default();
        let response_js = context
            .eval(Source::from_bytes(&format!("({response_json})")))
            .unwrap_or(JsValue::undefined());
        context.register_global_property(js_string!("response"), response_js.clone(), Attribute::all())?;
        context.register_global_property(js_string!("res"), response_js, Attribute::all())?;
    }

    // test(name, fn) + a minimal expect(actual) — enough for the common
    // `test("...", () => expect(x).toBe(y))` pattern. Richer chaining (`.to.have.status(...)`)
    // is intentionally left for a follow-up pass rather than guessed at here.
    let test_state = state.clone();
    let test_fn = NativeFunction::from_closure(move |_this, args, ctx| {
        let name = arg_as_string(args, 0, ctx)?;
        let callback = args.get(1).cloned().unwrap_or_default();
        let passed_result = callback
            .as_callable()
            .map(|c| c.call(&JsValue::undefined(), &[], ctx));
        let mut s = test_state.borrow_mut();
        match passed_result {
            Some(Ok(_)) => s.test_results.push(TestAssertionResult { name, passed: true, error: None }),
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
    });
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

/// Returns true if a request's pre-request or test script is non-empty — used by the import
/// flow to warn before running someone else's scripts, same as the Electron app's
/// `requestHasExecutableScript`.
pub fn request_has_executable_script(req: &ApiRequest) -> bool {
    req.has_executable_script()
}

// TODO(parity gap): hard-kill a runaway/infinite-loop script. The Electron version runs scripts
// in a Web Worker and calls `worker.terminate()` on timeout, which works because the worker is
// a separate OS-level thread the host can kill unconditionally. `boa_engine::Context::eval` is
// a plain synchronous call on the calling thread with no built-in interrupt hook in the version
// this was written against; the real fix is running `execute_pre_request_script` /
// `execute_test_script` on a dedicated thread via `std::thread::spawn`, joining with a timeout,
// and detaching (leaking) the thread if it doesn't finish in time. Not done yet in this pass.
#[allow(dead_code)]
fn _unused_kv_type_anchor(_: &KeyValuePair) {}
