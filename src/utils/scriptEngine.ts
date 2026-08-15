import { ApiRequest, ApiResponse, Environment, TestAssertionResult } from '../types';

export interface ScriptExecutionResult {
  testResults: TestAssertionResult[];
  logs: string[];
  updatedEnvVars: Record<string, string>;
}

// Chai/Jest-style expect assertion builder compatible with Postman pm.expect()
function createExpect(actual: any) {
  const self: any = {
    to: {
      equal(expected: any) {
        if (actual !== expected) {
          throw new Error(`Expected ${JSON.stringify(expected)} but got ${JSON.stringify(actual)}`);
        }
      },
      eql(expected: any) {
        if (JSON.stringify(actual) !== JSON.stringify(expected)) {
          throw new Error(`Expected ${JSON.stringify(expected)} but got ${JSON.stringify(actual)}`);
        }
      },
      be: {
        below(max: number) {
          if (typeof actual !== 'number' || actual >= max) {
            throw new Error(`Expected ${actual} to be below ${max}`);
          }
        },
        above(min: number) {
          if (typeof actual !== 'number' || actual <= min) {
            throw new Error(`Expected ${actual} to be above ${min}`);
          }
        },
        oneOf(list: any[]) {
          if (!list.includes(actual)) {
            throw new Error(`Expected ${JSON.stringify(actual)} to be one of ${JSON.stringify(list)}`);
          }
        },
        true: () => {
          if (actual !== true) throw new Error(`Expected true but got ${actual}`);
        },
        false: () => {
          if (actual !== false) throw new Error(`Expected false but got ${actual}`);
        },
        null: () => {
          if (actual !== null) throw new Error(`Expected null but got ${actual}`);
        },
        undefined: () => {
          if (actual !== undefined) throw new Error(`Expected undefined but got ${actual}`);
        },
        a(type: string) {
          if (typeof actual !== type) throw new Error(`Expected type ${type} but got ${typeof actual}`);
        },
        an(type: string) {
          if (typeof actual !== type) throw new Error(`Expected type ${type} but got ${typeof actual}`);
        }
      },
      have: {
        property(propName: string, expectedVal?: any) {
          if (!actual || !(propName in actual)) {
            throw new Error(`Expected object to have property "${propName}"`);
          }
          if (expectedVal !== undefined && actual[propName] !== expectedVal) {
            throw new Error(`Expected property "${propName}" to equal ${expectedVal}, got ${actual[propName]}`);
          }
        },
        header(headerName: string, expectedVal?: string) {
          const lowerName = headerName.toLowerCase();
          const headers = actual?.headers || actual;
          const found = Object.keys(headers || {}).find(k => k.toLowerCase() === lowerName);
          if (!found) {
            throw new Error(`Expected header "${headerName}" to exist`);
          }
          if (expectedVal !== undefined && headers[found] !== expectedVal) {
            throw new Error(`Expected header "${headerName}" to equal "${expectedVal}", got "${headers[found]}"`);
          }
        },
        status(code: number) {
          const status = actual?.status ?? actual;
          if (status !== code) {
            throw new Error(`Expected status ${code}, got ${status}`);
          }
        },
        lengthOf(len: number) {
          if (!actual || actual.length !== len) {
            throw new Error(`Expected length of ${len}, got ${actual?.length}`);
          }
        }
      },
      include(item: any) {
        if (typeof actual === 'string') {
          if (!actual.includes(item)) {
            throw new Error(`Expected "${actual}" to include "${item}"`);
          }
        } else if (Array.isArray(actual)) {
          if (!actual.includes(item)) {
            throw new Error(`Expected array to include ${JSON.stringify(item)}`);
          }
        }
      }
    }
  };
  return self;
}

// Executes Postman Pre-Request Scripts (before sending HTTP call)
export function executePreRequestScript(
  script: string,
  request: ApiRequest,
  env: Environment
): ScriptExecutionResult {
  const logs: string[] = [];
  const updatedEnvVars: Record<string, string> = {};

  if (!script || !script.trim()) {
    return { testResults: [], logs: [], updatedEnvVars: {} };
  }

  // Create environment variable accessor map
  const varMap: Record<string, string> = {};
  env.variables.forEach(v => {
    varMap[v.key] = v.value;
  });

  const customConsole = {
    log: (...args: any[]) => {
      logs.push(args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' '));
    },
    warn: (...args: any[]) => {
      logs.push(`[WARN] ` + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' '));
    },
    error: (...args: any[]) => {
      logs.push(`[ERROR] ` + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' '));
    }
  };

  const pm = {
    environment: {
      get: (key: string) => varMap[key] || '',
      set: (key: string, value: any) => {
        const valStr = String(value);
        varMap[key] = valStr;
        updatedEnvVars[key] = valStr;
      },
      has: (key: string) => key in varMap,
      unset: (key: string) => {
        delete varMap[key];
        delete updatedEnvVars[key];
      }
    },
    variables: {
      get: (key: string) => varMap[key] || '',
      set: (key: string, value: any) => {
        const valStr = String(value);
        varMap[key] = valStr;
        updatedEnvVars[key] = valStr;
      }
    },
    request: {
      url: request.url,
      method: request.method,
      headers: {
        add: (headerObj: { key: string; value: string }) => {
          logs.push(`Header added via script: ${headerObj.key}: ${headerObj.value}`);
        }
      }
    }
  };

  try {
    const fn = new Function('pm', 'console', 'environment', script);
    fn(pm, customConsole, pm.environment);
  } catch (err: any) {
    logs.push(`[Script Error]: ${err.message}`);
  }

  return { testResults: [], logs, updatedEnvVars };
}

// Executes Postman Tests & Post-Response Scripts (after HTTP response is received)
export function executeTestScript(
  script: string,
  response: ApiResponse,
  request: ApiRequest,
  env: Environment
): ScriptExecutionResult {
  const testResults: TestAssertionResult[] = [];
  const logs: string[] = [];
  const updatedEnvVars: Record<string, string> = {};

  if (!script || !script.trim()) {
    return { testResults: [], logs: [], updatedEnvVars: {} };
  }

  const varMap: Record<string, string> = {};
  env.variables.forEach(v => {
    varMap[v.key] = v.value;
  });

  const customConsole = {
    log: (...args: any[]) => {
      logs.push(args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' '));
    },
    warn: (...args: any[]) => {
      logs.push(`[WARN] ` + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' '));
    },
    error: (...args: any[]) => {
      logs.push(`[ERROR] ` + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' '));
    }
  };

  const test = (testName: string, callback: () => void) => {
    try {
      callback();
      testResults.push({ name: testName, passed: true });
    } catch (err: any) {
      testResults.push({ name: testName, passed: false, error: err.message || 'Assertion failed' });
    }
  };

  const pm = {
    test,
    expect: createExpect,
    environment: {
      get: (key: string) => varMap[key] || '',
      set: (key: string, value: any) => {
        const valStr = String(value);
        varMap[key] = valStr;
        updatedEnvVars[key] = valStr;
      },
      has: (key: string) => key in varMap
    },
    variables: {
      get: (key: string) => varMap[key] || '',
      set: (key: string, value: any) => {
        const valStr = String(value);
        varMap[key] = valStr;
        updatedEnvVars[key] = valStr;
      }
    },
    response: {
      code: response.status,
      status: response.statusText,
      responseTime: response.timeMs,
      headers: response.headers || {},
      json: () => response.data,
      text: () => typeof response.data === 'string' ? response.data : JSON.stringify(response.data),
      to: {
        have: {
          status: (code: number) => {
            if (response.status !== code) {
              throw new Error(`Expected status ${code}, got ${response.status}`);
            }
          },
          header: (headerKey: string, val?: string) => {
            const lowerKey = headerKey.toLowerCase();
            const found = Object.keys(response.headers || {}).find(k => k.toLowerCase() === lowerKey);
            if (!found) {
              throw new Error(`Expected response to have header "${headerKey}"`);
            }
            if (val !== undefined && response.headers[found] !== val) {
              throw new Error(`Expected header "${headerKey}" to equal "${val}", got "${response.headers[found]}"`);
            }
          }
        },
        be: {
          ok: () => {
            if (response.status < 200 || response.status >= 300) {
              throw new Error(`Expected 2xx status, got ${response.status}`);
            }
          },
          clientError: () => {
            if (response.status < 400 || response.status >= 500) {
              throw new Error(`Expected 4xx status, got ${response.status}`);
            }
          },
          serverError: () => {
            if (response.status < 500) {
              throw new Error(`Expected 5xx status, got ${response.status}`);
            }
          }
        }
      }
    }
  };

  try {
    const fn = new Function('pm', 'test', 'console', 'response', script);
    fn(pm, test, customConsole, response);
  } catch (err: any) {
    testResults.push({
      name: 'Script Syntax / Execution',
      passed: false,
      error: `Syntax Error: ${err.message}`
    });
  }

  return { testResults, logs, updatedEnvVars };
}
