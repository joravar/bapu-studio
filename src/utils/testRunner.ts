import { ApiResponse, TestAssertionResult } from '../types';

export function runApiTests(script: string, response: ApiResponse): TestAssertionResult[] {
  if (!script || !script.trim()) {
    return [];
  }

  const results: TestAssertionResult[] = [];

  // Create test context environment
  const test = (name: string, fn: () => void) => {
    try {
      fn();
      results.push({ name, passed: true });
    } catch (err: any) {
      results.push({ name, passed: false, error: err.message || 'Assertion failed' });
    }
  };

  const expect = (actual: any) => ({
    toBe(expected: any) {
      if (actual !== expected) {
        throw new Error(`Expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
      }
    },
    toEqual(expected: any) {
      if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`Expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
      }
    },
    toBeLessThan(expected: number) {
      if (typeof actual !== 'number' || actual >= expected) {
        throw new Error(`Expected ${actual} to be less than ${expected}`);
      }
    },
    toBeGreaterThan(expected: number) {
      if (typeof actual !== 'number' || actual <= expected) {
        throw new Error(`Expected ${actual} to be greater than ${expected}`);
      }
    },
    toContain(expected: string | any) {
      if (typeof actual === 'string') {
        if (!actual.includes(expected)) {
          throw new Error(`Expected string to contain "${expected}"`);
        }
      } else if (Array.isArray(actual)) {
        if (!actual.includes(expected)) {
          throw new Error(`Expected array to contain ${JSON.stringify(expected)}`);
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
    }
  });

  // Alias pm and bapu test syntax for Postman compatibility
  const pm = {
    test,
    expect,
    response: {
      to: {
        have: {
          status: (code: number) => {
            if (response.status !== code) {
              throw new Error(`Expected status ${code}, got ${response.status}`);
            }
          }
        }
      }
    }
  };

  const bapu = {
    test,
    expect,
    response
  };

  const res = {
    status: response.status,
    statusText: response.statusText,
    timeMs: response.timeMs,
    headers: response.headers || {},
    data: response.data,
    text: typeof response.data === 'string' ? response.data : JSON.stringify(response.data)
  };

  try {
    // Safe execution wrapper
    const runner = new Function('test', 'expect', 'res', 'response', 'pm', 'bapu', script);
    runner(test, expect, res, response, pm, bapu);
  } catch (err: any) {
    results.push({
      name: 'Script Syntax / Execution',
      passed: false,
      error: `Execution Error: ${err.message}`
    });
  }

  return results;
}
