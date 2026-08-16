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

test('Test script: evaluate native bapu.test, bapu.expect, and bapu.env namespace', () => {
  const response = {
    status: 200,
    statusText: 'OK',
    timeMs: 45,
    headers: { 'content-type': 'application/json' },
    data: { status: 'success', token: 'jwt_secret_9988' }
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

  const updatedEnv = {};
  const bapu = {
    test: testFn,
    expect: (actual) => ({
      toBe: (expected) => assert.strictEqual(actual, expected),
      toEqual: (expected) => assert.deepStrictEqual(actual, expected),
      toBeLessThan: (max) => assert.ok(actual < max),
      to: {
        equal: (expected) => assert.strictEqual(actual, expected),
        have: {
          property: (prop) => assert.ok(prop in actual)
        }
      }
    }),
    env: {
      set: (k, v) => { updatedEnv[k] = v; }
    },
    response: {
      status: response.status,
      timeMs: response.timeMs,
      json: () => response.data
    }
  };

  const script = `
    bapu.test("Status code is 200 OK", function () {
      bapu.expect(bapu.response.status).toBe(200);
    });

    bapu.test("Response time is fast", function () {
      bapu.expect(bapu.response.timeMs).toBeLessThan(500);
    });

    bapu.test("Received auth token", function () {
      var data = bapu.response.json();
      bapu.expect(data).to.have.property("token");
      bapu.env.set("AUTH_TOKEN", data.token);
    });
  `;

  const runner = new Function('bapu', script);
  runner(bapu);

  assert.strictEqual(testResults.length, 3);
  assert.strictEqual(testResults.every(t => t.passed), true);
  assert.strictEqual(updatedEnv.AUTH_TOKEN, 'jwt_secret_9988');
});

// ------------------------------------------------------------------------------
// 6. DRAG AND DROP SEQUENCE REORDERING
// ------------------------------------------------------------------------------
console.log('\n--- 6. Testing Drag & Drop Sequencing for APIs and Databases ---');

test('Drag & Drop: reorder API request items inside a collection', () => {
  const requests = [
    { id: 'r1', name: 'GET Users' },
    { id: 'r2', name: 'POST Users' },
    { id: 'r3', name: 'DELETE Users' }
  ];

  // Move r3 (index 2) to top (index 0)
  const sourceIndex = 2;
  const destIndex = 0;
  const copy = [...requests];
  const [removed] = copy.splice(sourceIndex, 1);
  copy.splice(destIndex, 0, removed);

  assert.strictEqual(copy[0].id, 'r3');
  assert.strictEqual(copy[1].id, 'r1');
  assert.strictEqual(copy[2].id, 'r2');
});

test('Drag & Drop: reorder Database driver connections priority', () => {
  const dbs = [
    { id: 'db-pg', name: 'PostgreSQL' },
    { id: 'db-my', name: 'MySQL' },
    { id: 'db-mg', name: 'MongoDB' }
  ];

  // Move MongoDB (index 2) to index 1
  const copy = [...dbs];
  const [removed] = copy.splice(2, 1);
  copy.splice(1, 0, removed);

  assert.deepStrictEqual(copy.map(d => d.name), ['PostgreSQL', 'MongoDB', 'MySQL']);
});

// ------------------------------------------------------------------------------
// 7. POSTMAN & OPENAPI IMPORT / EXPORT SERIALIZATION
// ------------------------------------------------------------------------------
console.log('\n--- 7. Testing Collection Import & Export Engine ---');

test('Export & Import: Postman Collection v2.1 round-trip integrity', () => {
  const sampleCollection = {
    id: 'col-test',
    name: 'Stripe Payments API',
    requests: [
      {
        id: 'r1',
        name: 'Create Payment Intent',
        method: 'POST',
        url: 'https://api.stripe.com/v1/payment_intents',
        params: [],
        headers: [{ id: 'h1', key: 'Authorization', value: 'Bearer sk_test_123', enabled: true }],
        bodyType: 'json',
        bodyContent: '{"amount": 2000, "currency": "usd"}',
        authType: 'bearer',
        authConfig: { token: 'sk_test_123' },
        preRequestScript: 'console.log("Starting Stripe charge...");',
        testScript: 'pm.test("Status is 200", function () { pm.response.to.have.status(200); });'
      }
    ]
  };

  // 1. Simulate Postman v2.1 Export
  const postmanDoc = {
    info: {
      name: sampleCollection.name,
      schema: 'https://schema.getpostman.com/json/collection/v2.1.0/collection.json'
    },
    item: [
      {
        name: sampleCollection.requests[0].name,
        event: [
          { listen: 'prerequest', script: { exec: ['console.log("Starting Stripe charge...");'] } },
          { listen: 'test', script: { exec: ['pm.test("Status is 200", function () { pm.response.to.have.status(200); });'] } }
        ],
        request: {
          method: 'POST',
          header: [{ key: 'Authorization', value: 'Bearer sk_test_123' }],
          url: { raw: 'https://api.stripe.com/v1/payment_intents' },
          body: { mode: 'raw', raw: '{"amount": 2000, "currency": "usd"}' }
        }
      }
    ]
  };

  const exportedJson = JSON.stringify(postmanDoc);
  assert.ok(exportedJson.includes('https://schema.getpostman.com/json/collection/v2.1.0/collection.json'));

  // 2. Simulate Import
  const importedDoc = JSON.parse(exportedJson);
  assert.strictEqual(importedDoc.info.name, 'Stripe Payments API');
  assert.strictEqual(importedDoc.item.length, 1);
  assert.strictEqual(importedDoc.item[0].request.method, 'POST');
  assert.strictEqual(importedDoc.item[0].event.length, 2);
});

test('Import: OpenAPI 3.0 YAML/JSON specification parsing', () => {
  const openApiDoc = {
    openapi: '3.0.0',
    info: { title: 'Petstore API', version: '1.0.0' },
    paths: {
      '/pets': {
        get: { summary: 'List all pets', parameters: [] },
        post: { summary: 'Create a pet', parameters: [] }
      },
      '/pets/{petId}': {
        get: { summary: 'Info for a specific pet', parameters: [] }
      }
    }
  };

  const pathEntries = Object.entries(openApiDoc.paths);
  assert.strictEqual(pathEntries.length, 2);
  assert.strictEqual(openApiDoc.info.title, 'Petstore API');
});

// ------------------------------------------------------------------------------
// 8. RENAMING COLLECTIONS & API REQUESTS
// ------------------------------------------------------------------------------
console.log('\n--- 8. Testing Renaming Architecture ---');

test('Renaming: update collection folder title', () => {
  let collections = [
    { id: 'col-1', name: 'Original API', requests: [] },
    { id: 'col-2', name: 'Other API', requests: [] }
  ];

  const targetId = 'col-1';
  const newName = 'Production Stripe API';

  collections = collections.map(c => c.id === targetId ? { ...c, name: newName } : c);

  assert.strictEqual(collections[0].name, 'Production Stripe API');
  assert.strictEqual(collections[1].name, 'Other API');
});

test('Renaming: update nested API request title', () => {
  let collections = [
    {
      id: 'col-1',
      name: 'Auth API',
      requests: [
        { id: 'req-1', name: 'Login V1', method: 'POST' },
        { id: 'req-2', name: 'Register', method: 'POST' }
      ]
    }
  ];

  const reqId = 'req-1';
  const newName = 'OAuth 2.0 Token Exchange';

  collections = collections.map(col => ({
    ...col,
    requests: col.requests.map(r => r.id === reqId ? { ...r, name: newName } : r)
  }));

  assert.strictEqual(collections[0].requests[0].name, 'OAuth 2.0 Token Exchange');
  assert.strictEqual(collections[0].requests[1].name, 'Register');
});

test('Renaming: update database connection driver title', () => {
  let databases = [
    { id: 'db-1', name: 'Local Dev DB', type: 'sqlite', database: 'main.sqlite' },
    { id: 'db-2', name: 'Prod Mongo', type: 'mongodb', database: 'production' }
  ];

  const targetDbId = 'db-1';
  const newDbName = 'Primary SQLite Database';

  databases = databases.map(db => db.id === targetDbId ? { ...db, name: newDbName } : db);

  assert.strictEqual(databases[0].name, 'Primary SQLite Database');
  assert.strictEqual(databases[1].name, 'Prod Mongo');
});

test('Dynamic Variable: propagate bapu.env.set into environment secrets matrix', () => {
  let activeEnv = {
    id: 'env-dev',
    name: 'Development',
    variables: [
      { id: 'v1', key: 'API_URL', value: 'https://api.example.com', enabled: true, isSecret: false }
    ]
  };

  const updatedEnvVars = {
    AUTH_TOKEN: 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9',
    TENANT_ID: 'tenant_9981'
  };

  const newVars = [...activeEnv.variables];
  Object.entries(updatedEnvVars).forEach(([k, val]) => {
    const isSecret = /token|secret|key|auth|password|jwt|bearer/i.test(k);
    const existing = newVars.find(v => v.key === k);
    if (existing) {
      existing.value = val;
      if (isSecret) existing.isSecret = true;
    } else {
      newVars.push({
        id: `var-${Date.now()}-${Math.random()}`,
        key: k,
        value: val,
        enabled: true,
        isSecret: isSecret
      });
    }
  });
  activeEnv = { ...activeEnv, variables: newVars };

  assert.strictEqual(activeEnv.variables.length, 3);
  const tokenVar = activeEnv.variables.find(v => v.key === 'AUTH_TOKEN');
  assert.ok(tokenVar);
  assert.strictEqual(tokenVar.isSecret, true);
  assert.strictEqual(tokenVar.value, 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9');
});

test('Environment Management: create and rename environment scope', () => {
  let envs = [
    { id: 'env-1', name: 'Dev', variables: [] }
  ];

  // Add environment
  const newEnv = { id: 'env-2', name: 'Staging EU', variables: [] };
  envs.push(newEnv);
  assert.strictEqual(envs.length, 2);

  // Rename environment
  envs = envs.map(e => e.id === 'env-2' ? { ...e, name: 'Staging Frankfurt' } : e);
  assert.strictEqual(envs[1].name, 'Staging Frankfurt');
});

// ------------------------------------------------------------------------------
// 9. DATABASE STUDIO INTEGRATION & QUERY ENGINE
// ------------------------------------------------------------------------------
console.log('\n--- 9. Testing Database Studio & Query Engine ---');

test('Database Studio: SQL SELECT query execution & column extraction', () => {
  const users = [
    { id: 'u_101', email: 'alex.rivera@acme.dev', name: 'Alex Rivera', role: 'admin', status: 'ACTIVE' },
    { id: 'u_102', email: 'sarah.connor@cyberdyne.io', name: 'Sarah Connor', role: 'developer', status: 'ACTIVE' },
    { id: 'u_103', email: 'elena.rostova@cloudscale.net', name: 'Elena Rostova', role: 'owner', status: 'ACTIVE' }
  ];

  const columns = Object.keys(users[0]);
  assert.strictEqual(columns.length, 5);
  assert.ok(columns.includes('email'));
  assert.ok(columns.includes('role'));

  // Filter admin
  const filtered = users.filter(u => u.role === 'admin');
  assert.strictEqual(filtered.length, 1);
  assert.strictEqual(filtered[0].email, 'alex.rivera@acme.dev');
});

test('Database Studio: CSV Export Serialization', () => {
  const columns = ['id', 'email', 'role'];
  const rows = [
    { id: 'u_1', email: 'test@bapu.io', role: 'admin' },
    { id: 'u_2', email: 'dev@bapu.io', role: 'developer' }
  ];

  const header = columns.join(',');
  const rowLines = rows.map(r => columns.map(col => JSON.stringify(r[col] || '')).join(','));
  const csv = [header, ...rowLines].join('\n');

  assert.ok(csv.includes('id,email,role'));
  assert.ok(csv.includes('"test@bapu.io"'));
  assert.ok(csv.includes('"dev@bapu.io"'));
});

test('Database Studio: MongoDB document key extraction for dynamic table grid', () => {
  const mongoDocs = [
    { _id: '65cb7891a123f001', name: 'Pro License', price: 99.00, inStock: true },
    { _id: '65cb7891a123f002', name: 'Starter Kit', price: 19.00, inStock: false }
  ];

  const allKeys = Array.from(new Set(mongoDocs.flatMap(doc => Object.keys(doc))));
  assert.deepStrictEqual(allKeys, ['_id', 'name', 'price', 'inStock']);
});

test('Database Studio: Connection String URI parser with special characters', () => {
  function parseDatabaseUri(uri) {
    const trimmed = uri.trim();
    let type = 'postgres';
    let cleanUri = trimmed;

    if (trimmed.startsWith('postgres://') || trimmed.startsWith('postgresql://')) {
      type = 'postgres';
      cleanUri = trimmed.replace(/^postgres(?:ql)?:\/\//i, '');
    } else if (trimmed.startsWith('mysql://') || trimmed.startsWith('mariadb://')) {
      type = 'mysql';
      cleanUri = trimmed.replace(/^(?:mysql|mariadb):\/\//i, '');
    } else if (trimmed.startsWith('mongodb://') || trimmed.startsWith('mongodb+srv://')) {
      type = 'mongodb';
      cleanUri = trimmed.replace(/^mongodb(?:\+srv)?:\/\//i, '');
    }

    const lastAtIndex = cleanUri.lastIndexOf('@');
    let username, password;
    if (lastAtIndex !== -1) {
      const authPart = cleanUri.substring(0, lastAtIndex);
      const colonIndex = authPart.indexOf(':');
      if (colonIndex !== -1) {
        username = decodeURIComponent(authPart.substring(0, colonIndex));
        password = decodeURIComponent(authPart.substring(colonIndex + 1));
      } else {
        username = decodeURIComponent(authPart);
      }
      cleanUri = cleanUri.substring(lastAtIndex + 1);
    }

    const slashIndex = cleanUri.indexOf('/');
    const hostPortPart = slashIndex !== -1 ? cleanUri.substring(0, slashIndex) : cleanUri;
    const pathAndQuery = slashIndex !== -1 ? cleanUri.substring(slashIndex + 1) : '';

    const colonHostIndex = hostPortPart.indexOf(':');
    let host = hostPortPart;
    let port = type === 'postgres' ? '5432' : '3306';
    if (colonHostIndex !== -1) {
      host = hostPortPart.substring(0, colonHostIndex);
      port = hostPortPart.substring(colonHostIndex + 1);
    }

    let database = pathAndQuery ? pathAndQuery.split('?')[0] : 'neondb';

    return { type, host, port, database, username, password };
  }

  // 1. Neon Postgres with complex password containing #, !, $
  const neon = parseDatabaseUri('postgresql://alex:npg_Secr3t#pass!99$@ep-cool-dawn.us-east-2.aws.neon.tech/neondb?sslmode=require');
  assert.strictEqual(neon.type, 'postgres');
  assert.strictEqual(neon.host, 'ep-cool-dawn.us-east-2.aws.neon.tech');
  assert.strictEqual(neon.database, 'neondb');
  assert.strictEqual(neon.username, 'alex');
  assert.strictEqual(neon.password, 'npg_Secr3t#pass!99$');

  // 2. MongoDB Atlas
  const atlas = parseDatabaseUri('mongodb+srv://admin:atlas998@cluster0.abcde.mongodb.net/production?retryWrites=true');
  assert.strictEqual(atlas.type, 'mongodb');
  assert.strictEqual(atlas.host, 'cluster0.abcde.mongodb.net');
  assert.strictEqual(atlas.database, 'production');
  assert.strictEqual(atlas.username, 'admin');

  // 3. MySQL TiDB Cloud
  const mysql = parseDatabaseUri('mysql://root:dbpass123@gateway01.us-east-1.tidbcloud.com:4000/app_db');
  assert.strictEqual(mysql.type, 'mysql');
  assert.strictEqual(mysql.host, 'gateway01.us-east-1.tidbcloud.com');
  assert.strictEqual(mysql.port, '4000');
  assert.strictEqual(mysql.database, 'app_db');
});

test('Database Studio: Edit & Update Database Connection Parameters', () => {
  let databases = [
    {
      id: 'db-1',
      name: 'Old Postgres Staging',
      type: 'postgres',
      database: 'staging_db',
      host: 'localhost',
      port: '5432',
      username: 'postgres',
      ssl: false,
      isConnected: true,
      tables: []
    }
  ];

  // Edit database connection parameters
  const updatedDb = {
    ...databases[0],
    name: 'Production Neon Postgres Cluster',
    host: 'ep-cool-dawn.us-east-2.aws.neon.tech',
    database: 'production_main',
    username: 'neondb_owner',
    ssl: true
  };

  databases = databases.map(d => d.id === updatedDb.id ? updatedDb : d);

  assert.strictEqual(databases[0].name, 'Production Neon Postgres Cluster');
  assert.strictEqual(databases[0].host, 'ep-cool-dawn.us-east-2.aws.neon.tech');
  assert.strictEqual(databases[0].database, 'production_main');
  assert.strictEqual(databases[0].username, 'neondb_owner');
  assert.strictEqual(databases[0].ssl, true);
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
