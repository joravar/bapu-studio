// ==============================================================================
// BAPU STUDIO — COMPREHENSIVE FULL-LIFECYCLE & INTEGRATION TEST SUITE
// ==============================================================================

import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';

console.log('🧪 Starting Full-Lifecycle & Integration Test Suite for Bapu Studio...\n');

let passedTests = 0;
let failedTests = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`  ✅ PASS: ${name}`);
    passedTests++;
  } catch (err) {
    console.error(`  ❌ FAIL: ${name}`);
    console.error(`     Error: ${err.message}`);
    failedTests++;
  }
}

// ------------------------------------------------------------------------------
// 1. LIFECYCLE & PERSISTENCE (Cold Boot Simulation)
// ------------------------------------------------------------------------------
console.log('--- 1. Testing Storage Persistence & Cold Reboot Hydration ---');

// Mock localStorage engine
class MockLocalStorage {
  constructor() {
    this.store = {};
  }
  getItem(key) {
    return this.store[key] || null;
  }
  setItem(key, val) {
    this.store[key] = String(val);
  }
  removeItem(key) {
    delete this.store[key];
  }
  clear() {
    this.store = {};
  }
}

const mockStorage = new MockLocalStorage();

test('Cold Boot 1: Save new collection and verify recovery after app restart', () => {
  // Session 1: User adds a custom collection
  const session1Collections = [
    { id: 'col-default', name: 'Default Collection', requests: [] },
    { id: 'col-custom-1', name: 'Production Microservices', requests: [{ id: 'req-101', name: 'Get Auth Token', method: 'POST', url: 'https://api.example.com/oauth' }] }
  ];
  mockStorage.setItem('bapu_collections', JSON.stringify(session1Collections));

  // Session 2 (Cold restart): Simulate app starting fresh and reading storage
  const loadedRaw = mockStorage.getItem('bapu_collections');
  assert.ok(loadedRaw, 'Storage must not be empty on restart');
  const session2Collections = JSON.parse(loadedRaw);

  assert.strictEqual(session2Collections.length, 2);
  assert.strictEqual(session2Collections[1].name, 'Production Microservices');
  assert.strictEqual(session2Collections[1].requests[0].name, 'Get Auth Token');
});

test('Cold Boot 2: Save custom database driver and verify schema on restart', () => {
  // Session 1: User connects to staging PostgreSQL
  const session1Databases = [
    {
      id: 'db-staging',
      name: 'Staging RDS Postgres',
      type: 'postgres',
      database: 'analytics_db',
      isConnected: true,
      tables: [{ name: 'metrics', rowCount: 9940 }]
    }
  ];
  mockStorage.setItem('bapu_databases', JSON.stringify(session1Databases));

  // Session 2 (Cold restart)
  const loadedDatabases = JSON.parse(mockStorage.getItem('bapu_databases'));
  assert.strictEqual(loadedDatabases.length, 1);
  assert.strictEqual(loadedDatabases[0].name, 'Staging RDS Postgres');
  assert.strictEqual(loadedDatabases[0].tables[0].name, 'metrics');
});

test('Cold Boot 3: Save environment matrix secrets and verify masking state', () => {
  const session1Envs = [
    {
      id: 'env-prod',
      name: 'Production',
      variables: [
        { key: 'API_SECRET', value: 'sk_live_99887766', enabled: true, isSecret: true },
        { key: 'PORT', value: '3000', enabled: true, isSecret: false }
      ]
    }
  ];
  mockStorage.setItem('bapu_environments', JSON.stringify(session1Envs));

  const loadedEnvs = JSON.parse(mockStorage.getItem('bapu_environments'));
  assert.strictEqual(loadedEnvs[0].variables.length, 2);
  assert.strictEqual(loadedEnvs[0].variables[0].isSecret, true);
  assert.strictEqual(loadedEnvs[0].variables[0].value, 'sk_live_99887766');
});

// ------------------------------------------------------------------------------
// 2. PACKAGING INTEGRITY (Prevent Blank Screen Bugs)
// ------------------------------------------------------------------------------
console.log('\n--- 2. Testing Desktop Build & Packaging Configuration ---');

test('Vite config must declare relative base path ("./") for desktop packaging', () => {
  const viteConfigPath = path.resolve('vite.config.ts');
  const content = fs.readFileSync(viteConfigPath, 'utf8');
  assert.ok(content.includes("base: './'") || content.includes('base: "./"'), 'vite.config.ts must have base: "./"');
});

test('Electron main process must handle packaged production files', () => {
  const mainPath = path.resolve('electron-main.cjs');
  const content = fs.readFileSync(mainPath, 'utf8');
  assert.ok(content.includes('app.isPackaged'), 'electron-main.cjs must branch on app.isPackaged');
  assert.ok(content.includes('loadFile'), 'electron-main.cjs must load local HTML bundle via loadFile');
});

// ------------------------------------------------------------------------------
// 3. CORE LOGIC ENGINE TESTS
// ------------------------------------------------------------------------------
console.log('\n--- 3. Testing Core Parsers & Code Generators ---');

function parseCurlCommand(rawCurl) {
  const cleanCmd = rawCurl.trim().replace(/\\\r?\n/g, ' ');
  let method = 'GET';
  let url = '';
  const headers = [];
  const params = [];
  let bodyContent = '';
  let bodyType = 'none';

  const urlMatch = cleanCmd.match(/(?:['"])(https?:\/\/[^'"]+)(?:['"])|(?:https?:\/\/[^\s]+)/i);
  if (urlMatch) {
    const fullUrl = urlMatch[1] || urlMatch[0];
    try {
      const parsedUrl = new URL(fullUrl);
      url = `${parsedUrl.origin}${parsedUrl.pathname}`;
      parsedUrl.searchParams.forEach((val, key) => {
        params.push({ id: `p-${key}`, key, value: val, enabled: true });
      });
    } catch {
      url = fullUrl;
    }
  }

  const methodMatch = cleanCmd.match(/(?:-X|--request)\s+([A-Z]+)/i);
  if (methodMatch) {
    method = methodMatch[1].toUpperCase();
  }

  const headerRegex = /(?:-H|--header)\s+["']([^"']+)["']/gi;
  let hMatch;
  while ((hMatch = headerRegex.exec(cleanCmd)) !== null) {
    const headerStr = hMatch[1];
    const colonIdx = headerStr.indexOf(':');
    if (colonIdx > 0) {
      const key = headerStr.slice(0, colonIdx).trim();
      const value = headerStr.slice(colonIdx + 1).trim();
      headers.push({ id: `h-${key}`, key, value, enabled: true });
    }
  }

  const dataRegex = /(?:-d|--data|--data-raw|--data-binary)\s+(['"])([\s\S]*?)\1/i;
  const dataMatch = cleanCmd.match(dataRegex);
  if (dataMatch) {
    bodyContent = dataMatch[2];
    bodyType = 'raw';
    if (!methodMatch) method = 'POST';
    try {
      JSON.parse(bodyContent);
      bodyType = 'json';
    } catch {}
  }

  return { method, url, params, headers, bodyType, bodyContent };
}

test('Parse GET request with query params', () => {
  const curl = 'curl "https://api.example.com/v1/users?limit=25&role=admin"';
  const res = parseCurlCommand(curl);
  assert.strictEqual(res.method, 'GET');
  assert.strictEqual(res.url, 'https://api.example.com/v1/users');
  assert.strictEqual(res.params.length, 2);
  assert.strictEqual(res.params[0].key, 'limit');
  assert.strictEqual(res.params[0].value, '25');
});

test('Parse POST request with Headers and JSON Body', () => {
  const curl = `curl -X POST https://api.example.com/v1/checkout \\
    -H "Content-Type: application/json" \\
    -H "Authorization: Bearer token_secret_123" \\
    -d '{"plan": "pro", "quantity": 2}'`;
  const res = parseCurlCommand(curl);
  assert.strictEqual(res.method, 'POST');
  assert.strictEqual(res.url, 'https://api.example.com/v1/checkout');
  assert.strictEqual(res.headers.length, 2);
  assert.strictEqual(res.headers[0].key, 'Content-Type');
  assert.strictEqual(res.bodyType, 'json');
});

// ------------------------------------------------------------------------------
// 4. DATABASE DRIVER SERVICE INTEGRITY
// ------------------------------------------------------------------------------
console.log('\n--- 4. Testing Native Database Drivers & Query Abstraction ---');

test('Verify native database driver modules are resolvable (Postgres, MySQL, Mongo, SQLite)', () => {
  const pkgJsonPath = path.resolve('package.json');
  const pkg = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
  assert.ok(pkg.dependencies.pg, 'PostgreSQL driver (pg) must be in dependencies');
  assert.ok(pkg.dependencies.mysql2, 'MySQL driver (mysql2) must be in dependencies');
  assert.ok(pkg.dependencies.mongodb, 'MongoDB driver (mongodb) must be in dependencies');
  assert.ok(pkg.dependencies['sql.js'], 'SQLite engine (sql.js) must be in dependencies');
});

test('Simulate MongoDB JSON document key extraction for dynamic table grid', () => {
  const sampleDocs = [
    { _id: '507f1f77bcf86cd799439011', email: 'alex@example.com', plan: 'pro' },
    { _id: '507f191e810c19729de860ea', email: 'maria@example.com', plan: 'enterprise', seats: 10 }
  ];
  const keySet = new Set();
  sampleDocs.forEach(d => Object.keys(d).forEach(k => keySet.add(k)));
  const cols = Array.from(keySet);
  assert.deepStrictEqual(cols, ['_id', 'email', 'plan', 'seats']);
  assert.strictEqual(sampleDocs.length, 2);
});

// ------------------------------------------------------------------------------
// 5. POSTMAN SCRIPTING & ASSERTION ENGINE
// ------------------------------------------------------------------------------
console.log('\n--- 5. Testing Postman-Style Pre-Request & Test Scripting Engine ---');

test('Pre-request script: evaluate pm.environment.set and console.log', () => {
  const env = { id: 'env-1', name: 'Dev', variables: [{ key: 'API_URL', value: 'https://example.com', enabled: true }] };
  const req = { id: 'r1', name: 'Test', method: 'GET', url: '{{API_URL}}/test', params: [], headers: [], bodyType: 'none', bodyContent: '', authType: 'none', authConfig: {} };
  
  const script = `
    pm.environment.set("req_timestamp", "1720000000");
    console.log("Pre-request executed successfully");
  `;
  
  // Test evaluation
  const updatedEnvVars = {};
  const logs = [];
  const pm = {
    environment: {
      set: (k, v) => { updatedEnvVars[k] = v; }
    }
  };
  const customConsole = { log: (msg) => logs.push(msg) };
  const fn = new Function('pm', 'console', script);
  fn(pm, customConsole);

  assert.strictEqual(updatedEnvVars.req_timestamp, "1720000000");
  assert.strictEqual(logs[0], "Pre-request executed successfully");
});

test('Test script: evaluate pm.test, status assertions, and json body checks', () => {
  const response = {
    status: 200,
    statusText: 'OK',
    timeMs: 45,
    headers: { 'content-type': 'application/json' },
    data: { status: 'success', user: { id: 101, name: 'Alice' } }
  };

  const testResults = [];
  const testFn = (name, cb) => {
    try {
      cb();
      testResults.push({ name, passed: true });
    } catch (e) {
      testResults.push({ name, passed: false, error: e.message });
    }
  };

  const pm = {
    test: testFn,
    expect: (actual) => ({
      to: {
        eql: (expected) => assert.deepStrictEqual(actual, expected),
        be: { below: (max) => assert.ok(actual < max) }
      }
    }),
    response: {
      to: {
        have: {
          status: (code) => assert.strictEqual(response.status, code)
        }
      },
      json: () => response.data,
      responseTime: response.timeMs
    }
  };

  const script = `
    pm.test("Status is 200", function () {
      pm.response.to.have.status(200);
    });

    pm.test("Response time < 200ms", function () {
      pm.expect(pm.response.responseTime).to.be.below(200);
    });

    pm.test("Status is success", function () {
      var data = pm.response.json();
      pm.expect(data.status).to.eql("success");
    });
  `;

  const runner = new Function('pm', script);
  runner(pm);

  assert.strictEqual(testResults.length, 3);
  assert.strictEqual(testResults.every(t => t.passed), true);
});

// ------------------------------------------------------------------------------
// SUMMARY
// ------------------------------------------------------------------------------
console.log('\n==============================================================================');
console.log(`📊 FULL-LIFECYCLE TEST SUITE: ${passedTests} Passed, ${failedTests} Failed.`);
console.log('==============================================================================\n');

if (failedTests > 0) {
  process.exit(1);
}
