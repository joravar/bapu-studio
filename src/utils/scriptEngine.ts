import { ApiRequest, ApiResponse, Environment, TestAssertionResult } from '../types';

export interface ScriptExecutionResult {
  testResults: TestAssertionResult[];
  logs: string[];
  updatedEnvVars: Record<string, string>;
}

const SANDBOX_TIMEOUT_MS = 5000;

// Static, trusted harness that runs inside a dedicated Web Worker.
// The user's script text is NEVER embedded into this source — it only ever arrives at runtime as
// a postMessage string payload and is executed with `new Function`, so there's no injection surface.
// A worker has no `window`/`document`/`localStorage`/bapuBridge at all (those are Window-only APIs),
// so a malicious script can't reach app internals just by being inside this scope. `fetch`/XHR/
// WebSocket/importScripts are additionally stripped below so it can't exfiltrate anything it's
// legitimately handed (e.g. environment variable values). Crucially, a worker runs on its own OS
// thread: an infinite loop in the script can't freeze the app's UI thread, and `worker.terminate()`
// from the main thread can hard-kill it even mid-loop, unlike an iframe (which shares the main thread).
const WORKER_SOURCE = `
self.fetch = undefined;
self.XMLHttpRequest = undefined;
self.WebSocket = undefined;
self.EventSource = undefined;
self.importScripts = undefined;

(function () {
  function formatLog(args) {
    var time = new Date().toLocaleTimeString();
    var parts = args.map(function (a) {
      if (a && typeof a === 'object') {
        try { return JSON.stringify(a, null, 2); } catch (e) { return String(a); }
      }
      return String(a);
    });
    return '[' + time + '] ' + parts.join(' ');
  }

  function createExpect(actual) {
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
        if (typeof actual === 'string') {
          if (actual.indexOf(item) === -1) throw new Error('Expected "' + actual + '" to contain "' + item + '"');
        } else if (Array.isArray(actual)) {
          if (actual.indexOf(item) === -1) throw new Error('Expected array to contain ' + JSON.stringify(item));
        } else {
          throw new Error('Cannot check toContain on ' + typeof actual);
        }
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
          true: function () { if (actual !== true) throw new Error('Expected true but got ' + actual); },
          false: function () { if (actual !== false) throw new Error('Expected false but got ' + actual); },
          null: function () { if (actual !== null) throw new Error('Expected null but got ' + actual); },
          undefined: function () { if (actual !== undefined) throw new Error('Expected undefined but got ' + actual); },
          a: function (type) { if (typeof actual !== type) throw new Error('Expected type ' + type + ' but got ' + typeof actual); },
          an: function (type) { if (typeof actual !== type) throw new Error('Expected type ' + type + ' but got ' + typeof actual); }
        },
        have: {
          property: function (propName, expectedVal) {
            if (!actual || !(propName in actual)) throw new Error('Expected object to have property "' + propName + '"');
            if (expectedVal !== undefined && actual[propName] !== expectedVal) throw new Error('Expected property "' + propName + '" to equal ' + expectedVal + ', got ' + actual[propName]);
          },
          header: function (headerName, expectedVal) {
            var lowerName = headerName.toLowerCase();
            var headers = (actual && actual.headers) || actual;
            var found = Object.keys(headers || {}).find(function (k) { return k.toLowerCase() === lowerName; });
            if (!found) throw new Error('Expected header "' + headerName + '" to exist');
            if (expectedVal !== undefined && headers[found] !== expectedVal) throw new Error('Expected header "' + headerName + '" to equal "' + expectedVal + '", got "' + headers[found] + '"');
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
          if (typeof actual === 'string') {
            if (actual.indexOf(item) === -1) throw new Error('Expected "' + actual + '" to include "' + item + '"');
          } else if (Array.isArray(actual)) {
            if (actual.indexOf(item) === -1) throw new Error('Expected array to include ' + JSON.stringify(item));
          }
        }
      }
    };
  }

  self.onmessage = function (evt) {
    var msg = evt.data;
    if (!msg || msg.type !== 'init') return;

    var payload = msg.payload;
    var kind = payload.kind;
    var script = payload.script;
    var logs = [];
    var testResults = [];
    var updatedEnvVars = {};
    var varMap = {};
    (((payload.env || {}).variables) || []).forEach(function (v) {
      varMap[v.key] = v.value;
    });

    var customConsole = {
      log: function () { logs.push(formatLog(Array.prototype.slice.call(arguments))); },
      info: function () { logs.push(formatLog(Array.prototype.slice.call(arguments))); },
      warn: function () { logs.push(formatLog(['[WARN]'].concat(Array.prototype.slice.call(arguments)))); },
      error: function () { logs.push(formatLog(['[ERROR]'].concat(Array.prototype.slice.call(arguments)))); }
    };

    var envApi = {
      get: function (key) { return varMap[key] || ''; },
      set: function (key, value) { var v = String(value); varMap[key] = v; updatedEnvVars[key] = v; },
      has: function (key) { return key in varMap; },
      unset: function (key) { delete varMap[key]; delete updatedEnvVars[key]; }
    };

    try {
      if (kind === 'pre-request') {
        var reqShim = {
          url: payload.request.url,
          method: payload.request.method,
          headers: { add: function (h) { logs.push(formatLog(['Header added via script: ' + (h && h.key) + ': ' + (h && h.value)])); } }
        };
        var bapuPre = { log: customConsole.log, env: envApi, environment: envApi, variables: envApi, request: reqShim };
        var pmPre = { log: customConsole.log, environment: envApi, variables: envApi, request: reqShim };
        var preFn = new Function('bapu', 'pm', 'console', 'env', 'environment', 'request', 'req', script);
        preFn(bapuPre, pmPre, customConsole, envApi, envApi, reqShim, reqShim);
      } else {
        var test = function (name, cb) {
          try { cb(); testResults.push({ name: name, passed: true }); }
          catch (err) { testResults.push({ name: name, passed: false, error: (err && err.message) || 'Assertion failed' }); }
        };
        var respSrc = payload.response;
        var responseProxy = {
          status: respSrc.status,
          code: respSrc.status,
          statusText: respSrc.statusText,
          timeMs: respSrc.timeMs,
          responseTime: respSrc.timeMs,
          sizeBytes: respSrc.sizeBytes,
          headers: respSrc.headers || {},
          data: respSrc.data,
          json: function () { return respSrc.data; },
          text: function () { return typeof respSrc.data === 'string' ? respSrc.data : JSON.stringify(respSrc.data); },
          to: {
            have: {
              status: function (code) { if (respSrc.status !== code) throw new Error('Expected status ' + code + ', got ' + respSrc.status); },
              header: function (key, val) {
                var lowerKey = key.toLowerCase();
                var found = Object.keys(respSrc.headers || {}).find(function (k) { return k.toLowerCase() === lowerKey; });
                if (!found) throw new Error('Expected response to have header "' + key + '"');
                if (val !== undefined && respSrc.headers[found] !== val) throw new Error('Expected header "' + key + '" to equal "' + val + '", got "' + respSrc.headers[found] + '"');
              }
            },
            be: {
              ok: function () { if (respSrc.status < 200 || respSrc.status >= 300) throw new Error('Expected 2xx status, got ' + respSrc.status); },
              clientError: function () { if (respSrc.status < 400 || respSrc.status >= 500) throw new Error('Expected 4xx status, got ' + respSrc.status); },
              serverError: function () { if (respSrc.status < 500) throw new Error('Expected 5xx status, got ' + respSrc.status); }
            }
          }
        };
        var reqInfo = { url: payload.request.url, method: payload.request.method, headers: payload.request.headers };
        var bapu = { test: test, expect: createExpect, log: customConsole.log, env: envApi, environment: envApi, variables: envApi, response: responseProxy, request: reqInfo };
        var pm = { test: test, expect: createExpect, log: customConsole.log, environment: envApi, variables: envApi, response: responseProxy, request: reqInfo };
        var testFn = new Function('bapu', 'pm', 'test', 'expect', 'console', 'log', 'response', 'res', 'env', 'environment', script);
        testFn(bapu, pm, test, createExpect, customConsole, customConsole.log, responseProxy, responseProxy, envApi, envApi);
      }
    } catch (err) {
      if (kind === 'pre-request') {
        logs.push('[Script Error]: ' + ((err && err.message) || String(err)));
      } else {
        testResults.push({ name: 'Script Syntax / Execution', passed: false, error: 'Syntax Error: ' + ((err && err.message) || String(err)) });
      }
    }

    self.postMessage({ type: 'result', testResults: testResults, logs: logs, updatedEnvVars: updatedEnvVars });
  };
})();
`;

function runInSandbox(
  kind: 'pre-request' | 'test',
  script: string,
  payload: Record<string, any>
): Promise<ScriptExecutionResult> {
  return new Promise((resolve) => {
    if (!script || !script.trim()) {
      resolve({ testResults: [], logs: [], updatedEnvVars: {} });
      return;
    }

    const blobUrl = URL.createObjectURL(new Blob([WORKER_SOURCE], { type: 'application/javascript' }));
    const worker = new Worker(blobUrl);

    let settled = false;
    let timer: ReturnType<typeof setTimeout>;

    const cleanup = () => {
      clearTimeout(timer);
      worker.onmessage = null;
      worker.onerror = null;
      // terminate() kills the worker's thread outright, even mid-infinite-loop.
      worker.terminate();
      URL.revokeObjectURL(blobUrl);
    };

    const finish = (result: ScriptExecutionResult) => {
      if (settled) return;
      settled = true;
      cleanup();
      resolve(result);
    };

    worker.onmessage = (event: MessageEvent) => {
      const msg = event.data;
      if (!msg || msg.type !== 'result') return;
      finish({
        testResults: msg.testResults || [],
        logs: msg.logs || [],
        updatedEnvVars: msg.updatedEnvVars || {}
      });
    };

    worker.onerror = (event: ErrorEvent) => {
      event.preventDefault();
      const message = event.message || 'Unknown script error';
      finish({
        testResults: kind === 'test'
          ? [{ name: 'Script Execution', passed: false, error: message }]
          : [],
        logs: kind === 'pre-request' ? [`[Script Error]: ${message}`] : [],
        updatedEnvVars: {}
      });
    };

    timer = setTimeout(() => {
      finish({
        testResults: kind === 'test'
          ? [{ name: 'Script Execution', passed: false, error: `Script timed out after ${SANDBOX_TIMEOUT_MS}ms (possible infinite loop)` }]
          : [],
        logs: kind === 'pre-request'
          ? [`[Script Error]: Timed out after ${SANDBOX_TIMEOUT_MS}ms (possible infinite loop)`]
          : [],
        updatedEnvVars: {}
      });
    }, SANDBOX_TIMEOUT_MS);

    worker.postMessage({ type: 'init', payload });
  });
}

// Executes Pre-Request Scripts (before sending HTTP call) inside a sandboxed worker.
export function executePreRequestScript(
  script: string,
  request: ApiRequest,
  env: Environment
): Promise<ScriptExecutionResult> {
  return runInSandbox('pre-request', script, {
    kind: 'pre-request',
    script,
    request: { url: request.url, method: request.method, headers: request.headers },
    env: { variables: env.variables }
  });
}

// Executes Tests & Post-Response Scripts (after HTTP response is received) inside a sandboxed worker.
export function executeTestScript(
  script: string,
  response: ApiResponse,
  request: ApiRequest,
  env: Environment
): Promise<ScriptExecutionResult> {
  return runInSandbox('test', script, {
    kind: 'test',
    script,
    request: { url: request.url, method: request.method, headers: request.headers },
    response: {
      status: response.status,
      statusText: response.statusText,
      timeMs: response.timeMs,
      sizeBytes: response.sizeBytes,
      headers: response.headers || {},
      data: response.data
    },
    env: { variables: env.variables }
  });
}
