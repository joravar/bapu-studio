import { ApiRequest, ApiResponse, Environment, TestAssertionResult } from '../types';

export interface ScriptExecutionResult {
  testResults: TestAssertionResult[];
  logs: string[];
  updatedEnvVars: Record<string, string>;
}

// Chai + Jest compatible expect assertion builder
function createExpect(actual: any) {
  const self: any = {
    // Jest-style top-level matchers
    toBe(expected: any) {
      if (actual !== expected) {
        throw new Error(`Expected ${JSON.stringify(expected)} but got ${JSON.stringify(actual)}`);
      }
    },
    toEqual(expected: any) {
      if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`Expected ${JSON.stringify(expected)} but got ${JSON.stringify(actual)}`);
      }
    },
    toBeLessThan(max: number) {
      if (typeof actual !== 'number' || actual >= max) {
        throw new Error(`Expected ${actual} to be less than ${max}`);
      }
    },
    toBeGreaterThan(min: number) {
      if (typeof actual !== 'number' || actual <= min) {
        throw new Error(`Expected ${actual} to be greater than ${min}`);
      }
    },
    toContain(item: any) {
      if (typeof actual === 'string') {
        if (!actual.includes(item)) {
          throw new Error(`Expected "${actual}" to contain "${item}"`);
        }
      } else if (Array.isArray(actual)) {
        if (!actual.includes(item)) {
          throw new Error(`Expected array to contain ${JSON.stringify(item)}`);
        }
      } else {
        throw new Error(`Cannot check toContain on ${typeof actual}`);
      }
    },
    toBeDefined() {
      if (actual === undefined || actual === null) {
        throw new Error(`Expected value to be defined, got ${actual}`);
      }
    },
    toBeTruthy() {
      if (!actual) {
        throw new Error(`Expected truthy value, got ${actual}`);
      }
    },
    toBeFalsy() {
      if (actual) {
        throw new Error(`Expected falsy value, got ${actual}`);
      }
    },

    // Chai / Postman BDD-style chaining (pm.expect().to.equal(), to.have.property())
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
          const status = actual?.status ?? actual?.code ?? actual;
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

// Executes Pre-Request Scripts (before sending HTTP call)
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

  const varMap: Record<string, string> = {};
  env.variables.forEach(v => {
    varMap[v.key] = v.value;
  });

  const formatLog = (...args: any[]): string => {
    const time = new Date().toLocaleTimeString();
    const formatted = args.map(a => {
      if (typeof a === 'object') {
        try {
          return JSON.stringify(a, null, 2);
        } catch {
          return String(a);
        }
      }
      return String(a);
    }).join(' ');
    return `[${time}] ${formatted}`;
  };

  const customConsole = {
    log: (...args: any[]) => {
      logs.push(formatLog(...args));
    },
    info: (...args: any[]) => {
      logs.push(formatLog(...args));
    },
    warn: (...args: any[]) => {
      logs.push(formatLog('[WARN]', ...args));
    },
    error: (...args: any[]) => {
      logs.push(formatLog('[ERROR]', ...args));
    }
  };

  const envApi = {
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
  };

  const reqObj = {
    url: request.url,
    method: request.method,
    headers: {
      add: (headerObj: { key: string; value: string }) => {
        logs.push(formatLog(`Header added via script: ${headerObj.key}: ${headerObj.value}`));
      }
    }
  };

  const bapu = {
    log: customConsole.log,
    env: envApi,
    environment: envApi,
    variables: envApi,
    request: reqObj
  };

  const pm = {
    log: customConsole.log,
    environment: envApi,
    variables: envApi,
    request: reqObj
  };

  try {
    const fn = new Function(
      'bapu',
      'pm',
      'console',
      'env',
      'environment',
      'request',
      'req',
      script
    );
    fn(bapu, pm, customConsole, envApi, envApi, reqObj, reqObj);
  } catch (err: any) {
    logs.push(`[Script Error]: ${err.message}`);
  }

  return { testResults: [], logs, updatedEnvVars };
}

// Executes Tests & Post-Response Scripts (after HTTP response is received)
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

  const formatLog = (...args: any[]): string => {
    const time = new Date().toLocaleTimeString();
    const formatted = args.map(a => {
      if (typeof a === 'object') {
        try {
          return JSON.stringify(a, null, 2);
        } catch {
          return String(a);
        }
      }
      return String(a);
    }).join(' ');
    return `[${time}] ${formatted}`;
  };

  const customConsole = {
    log: (...args: any[]) => {
      logs.push(formatLog(...args));
    },
    info: (...args: any[]) => {
      logs.push(formatLog(...args));
    },
    warn: (...args: any[]) => {
      logs.push(formatLog('[WARN]', ...args));
    },
    error: (...args: any[]) => {
      logs.push(formatLog('[ERROR]', ...args));
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

  const envApi = {
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
  };

  const responseProxy = {
    status: response.status,
    code: response.status,
    statusText: response.statusText,
    timeMs: response.timeMs,
    responseTime: response.timeMs,
    sizeBytes: response.sizeBytes,
    headers: response.headers || {},
    data: response.data,
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
  };

  // Primary Bapu namespace
  const bapu = {
    test,
    expect: createExpect,
    log: customConsole.log,
    env: envApi,
    environment: envApi,
    variables: envApi,
    response: responseProxy,
    request: {
      url: request.url,
      method: request.method,
      headers: request.headers
    }
  };

  // Postman compatibility alias
  const pm = {
    test,
    expect: createExpect,
    log: customConsole.log,
    environment: envApi,
    variables: envApi,
    response: responseProxy,
    request: bapu.request
  };

  try {
    const fn = new Function(
      'bapu',
      'pm',
      'test',
      'expect',
      'console',
      'log',
      'response',
      'res',
      'env',
      'environment',
      script
    );
    fn(
      bapu,
      pm,
      test,
      createExpect,
      customConsole,
      customConsole.log,
      responseProxy,
      responseProxy,
      envApi,
      envApi
    );
  } catch (err: any) {
    testResults.push({
      name: 'Script Syntax / Execution',
      passed: false,
      error: `Syntax Error: ${err.message}`
    });
  }

  return { testResults, logs, updatedEnvVars };
}
