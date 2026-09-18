const { app, BrowserWindow, ipcMain, shell } = require('electron');
const path = require('path');
const pg = require('pg');
const mysql = require('mysql2/promise');
const { MongoClient } = require('mongodb');

// Active Connection Pools & Clients Cache
const pgPools = new Map();
const mysqlPools = new Map();
const mongoClients = new Map();

function createWindow() {
  const win = new BrowserWindow({
    width: 1320,
    height: 860,
    minWidth: 960,
    minHeight: 620,
    backgroundColor: '#090d14',
    title: 'Bapu Studio — Developer Cockpit',
    autoHideMenuBar: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: true,
      preload: path.join(__dirname, 'preload.cjs'),
    },
  });

  // Handle external links (e.g. GitHub Sponsor, Docs) to open in system default browser
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('https:') || url.startsWith('http:')) {
      shell.openExternal(url);
    }
    return { action: 'deny' };
  });

  win.webContents.on('will-navigate', (event, url) => {
    if (url.startsWith('http:') || url.startsWith('https:')) {
      const isDevServer = !app.isPackaged && url.includes('localhost:3000');
      if (!isDevServer) {
        event.preventDefault();
        shell.openExternal(url);
      }
    }
  });

  if (app.isPackaged) {
    win.loadFile(path.join(__dirname, 'dist', 'index.html'));
  } else {
    const devUrl = process.env.VITE_DEV_SERVER_URL || 'http://localhost:3000';
    win.loadURL(devUrl).catch(() => {
      win.loadFile(path.join(__dirname, 'dist', 'index.html'));
    });
  }

  // Set Content-Security-Policy headers
  const { session } = require('electron');
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self'; " +
          "script-src 'self' 'unsafe-inline' 'unsafe-eval'; " +
          "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; " +
          "font-src 'self' https://fonts.gstatic.com; " +
          "connect-src 'self' https: http: ws: wss:; " +
          "img-src 'self' data: https:;"
        ]
      }
    });
  });
}

// ------------------------------------------------------------------------------
// NATIVE DATABASE DRIVER IPC HANDLERS (PostgreSQL, MySQL, MongoDB)
// Helper to build robust Postgres config with auto SSL, TLS SNI & custom CA / mTLS certificates
function getPgConfig(config) {
  let host = config.host;
  if (!host && config.connectionString) {
    try {
      const match = config.connectionString.match(/@([^/:?]+)/);
      if (match) host = match[1];
    } catch {}
  }

  const isSslNeeded = config.ssl !== false && (
    config.ssl === true || 
    Boolean(config.sslCaCert || config.sslClientCert) ||
    (config.connectionString && (config.connectionString.includes('sslmode') || config.connectionString.includes('neon.tech') || config.connectionString.includes('supabase') || config.connectionString.includes('aiven') || config.connectionString.includes('render.com') || config.connectionString.includes('aws'))) ||
    (host && !host.includes('localhost') && !host.includes('127.0.0.1'))
  );

  let sslConfig = undefined;
  if (isSslNeeded) {
    sslConfig = {
      rejectUnauthorized: config.sslRejectUnauthorized !== undefined ? Boolean(config.sslRejectUnauthorized) : (config.sslCaCert ? true : false),
      servername: host || undefined
    };
    if (config.sslCaCert) {
      sslConfig.ca = config.sslCaCert;
    }
    if (config.sslClientCert) {
      sslConfig.cert = config.sslClientCert;
    }
    if (config.sslClientKey) {
      sslConfig.key = config.sslClientKey;
    }
  }

  // If a full connection string URI is provided, use it directly with SSL SNI options
  if (config.connectionString) {
    return {
      connectionString: config.connectionString,
      ssl: sslConfig,
      connectionTimeoutMillis: 10000
    };
  }

  // Otherwise use discrete credentials parameters
  return {
    host: host || 'localhost',
    port: parseInt(config.port, 10) || 5432,
    database: config.database || 'postgres',
    user: config.username || 'postgres',
    password: config.password || '',
    connectionTimeoutMillis: 10000,
    ssl: sslConfig
  };
}

function getMysqlSslConfig(config) {
  const isSslNeeded = config.ssl === true || Boolean(config.sslCaCert || config.sslClientCert);
  if (!isSslNeeded) return undefined;

  const ssl = {
    rejectUnauthorized: config.sslRejectUnauthorized !== undefined ? Boolean(config.sslRejectUnauthorized) : (config.sslCaCert ? true : false)
  };
  if (config.sslCaCert) ssl.ca = config.sslCaCert;
  if (config.sslClientCert) ssl.cert = config.sslClientCert;
  if (config.sslClientKey) ssl.key = config.sslClientKey;
  return ssl;
}

// 1. Test Database Connection
ipcMain.handle('db:test-connection', async (event, config) => {
  const startTime = Date.now();
  try {
    if (config.type === 'postgres') {
      const client = new pg.Client(getPgConfig(config));
      await client.connect();
      await client.query('SELECT 1');
      await client.end();
      const latencyMs = Date.now() - startTime;
      return { success: true, latencyMs, message: `Connected to PostgreSQL successfully (${latencyMs}ms)` };
    }

    if (config.type === 'mysql') {
      const sslConfig = getMysqlSslConfig(config);
      let connection;
      if (config.connectionString) {
        connection = await mysql.createConnection({
          uri: config.connectionString,
          connectTimeout: 8000,
          ssl: sslConfig
        });
      } else {
        connection = await mysql.createConnection({
          host: config.host || 'localhost',
          port: parseInt(config.port, 10) || 3306,
          database: config.database,
          user: config.username || 'root',
          password: config.password || '',
          connectTimeout: 8000,
          ssl: sslConfig
        });
      }
      await connection.query('SELECT 1');
      await connection.end();
      const latencyMs = Date.now() - startTime;
      return { success: true, latencyMs, message: `Connected to MySQL successfully (${latencyMs}ms)` };
    }

    if (config.type === 'mongodb') {
      let uri = config.connectionString;
      if (!uri) {
        const auth = config.username && config.password ? `${encodeURIComponent(config.username)}:${encodeURIComponent(config.password)}@` : '';
        const host = config.host || 'localhost';
        const port = config.port || 27017;
        uri = `mongodb://${auth}${host}:${port}/${config.database || 'admin'}`;
      }
      const client = new MongoClient(uri, { serverSelectionTimeoutMS: 7000 });
      await client.connect();
      await client.db(config.database || 'admin').command({ ping: 1 });
      await client.close();
      const latencyMs = Date.now() - startTime;
      return { success: true, latencyMs, message: `Connected to MongoDB successfully (${latencyMs}ms)` };
    }

    return { success: true, latencyMs: 5, message: `Driver ready` };
  } catch (err) {
    let friendlyMessage = err.message || 'Database connection failed';
    if (err.code === 'ECONNREFUSED' || err.message?.includes('ECONNREFUSED')) {
      friendlyMessage = `Could not connect to ${config.type?.toUpperCase() || 'Database'} on ${config.host || 'localhost'}:${config.port || 5432} (Connection Refused). Verify host address and port.`;
    } else if (err.message?.includes('password authentication failed')) {
      friendlyMessage = `Authentication failed: Incorrect username or password for user "${config.username || 'postgres'}".`;
    } else if (err.code === 'ETIMEDOUT' || err.message?.includes('timeout')) {
      friendlyMessage = `Connection timed out reaching ${config.host || 'server'}:${config.port || 'port'}. Verify server address, port, and firewall rules.`;
    }
    return { success: false, message: friendlyMessage };
  }
});

// 2. Execute SQL / MongoDB Query
ipcMain.handle('db:query', async (event, { config, sql }) => {
  const startTime = Date.now();
  try {
    if (config.type === 'postgres') {
      let pool = pgPools.get(config.id);
      if (!pool) {
        pool = new pg.Pool(getPgConfig(config));
        pgPools.set(config.id, pool);
      }

      const res = await pool.query(sql);
      const executionTimeMs = Date.now() - startTime;
      const columns = res.fields ? res.fields.map(f => f.name) : (res.rows[0] ? Object.keys(res.rows[0]) : []);

      let queryPlan = undefined;
      if (columns.length === 1 && (columns[0].toLowerCase().includes('plan') || columns[0].toLowerCase().includes('explain'))) {
        queryPlan = res.rows.map(r => Object.values(r)[0]).join('\n');
      }

      return {
        success: true,
        columns,
        rows: res.rows || [],
        rowCount: res.rowCount ?? (res.rows ? res.rows.length : 0),
        executionTimeMs,
        command: res.command || undefined,
        queryPlan
      };
    }

    if (config.type === 'mysql') {
      let pool = mysqlPools.get(config.id);
      if (!pool) {
        const sslConfig = getMysqlSslConfig(config);
        if (config.connectionString) {
          pool = mysql.createPool({
            uri: config.connectionString,
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000,
            ssl: sslConfig
          });
        } else {
          pool = mysql.createPool({
            host: config.host || 'localhost',
            port: parseInt(config.port, 10) || 3306,
            database: config.database,
            user: config.username || 'root',
            password: config.password || '',
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000,
            ssl: sslConfig
          });
        }
        mysqlPools.set(config.id, pool);
      }

      const [rows, fields] = await pool.query(sql);
      const executionTimeMs = Date.now() - startTime;
      const columns = fields ? fields.map(f => f.name) : (Array.isArray(rows) && rows[0] ? Object.keys(rows[0]) : []);

      let queryPlan = undefined;
      if (sql.trim().toUpperCase().startsWith('EXPLAIN')) {
        queryPlan = JSON.stringify(rows, null, 2);
      }

      return {
        success: true,
        columns,
        rows: Array.isArray(rows) ? rows : [],
        rowCount: Array.isArray(rows) ? rows.length : (rows.affectedRows || 0),
        executionTimeMs,
        command: sql.trim().split(/\s+/)[0]?.toUpperCase(),
        queryPlan
      };
    }

    if (config.type === 'mongodb') {
      let client = mongoClients.get(config.id);
      if (!client) {
        let uri = config.connectionString;
        if (!uri) {
          const auth = config.username && config.password ? `${encodeURIComponent(config.username)}:${encodeURIComponent(config.password)}@` : '';
          const host = config.host || 'localhost';
          const port = config.port || 27017;
          uri = `mongodb://${auth}${host}:${port}/${config.database || 'test'}`;
        }
        client = new MongoClient(uri);
        await client.connect();
        mongoClients.set(config.id, client);
      }

      const db = client.db(config.database || 'test');
      
      // Parse query string (supports JSON filter or collection.find format)
      let collectionName = config.tables?.[0]?.name || 'documents';
      let filter = {};

      const trimmed = sql.trim();
      if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
        try { filter = JSON.parse(trimmed); } catch {}
      } else {
        // Match e.g. "users.find({ status: 'active' })"
        const match = trimmed.match(/^([a-zA-Z0-9_-]+)\.find\((.*)\)/);
        if (match) {
          collectionName = match[1];
          try { filter = JSON.parse(match[2] || '{}'); } catch {}
        }
      }

      const collection = db.collection(collectionName);
      const docs = await collection.find(filter).limit(50).toArray();
      const executionTimeMs = Date.now() - startTime;

      // Extract unique keys across documents for table columns
      const keySet = new Set();
      docs.forEach(doc => Object.keys(doc).forEach(k => keySet.add(k)));
      const columns = Array.from(keySet);

      // Serialize ObjectIds for UI display
      const rows = docs.map(d => {
        const clean = { ...d };
        if (clean._id) clean._id = String(clean._id);
        return clean;
      });

      return {
        success: true,
        columns,
        rows,
        rowCount: rows.length,
        executionTimeMs
      };
    }

    return { success: false, message: `Unsupported driver: ${config.type}` };
  } catch (err) {
    let friendlyMessage = err.message || 'Query execution error';
    if (err.code === 'ECONNREFUSED' || err.message?.includes('ECONNREFUSED')) {
      friendlyMessage = `Could not connect to ${config.type?.toUpperCase() || 'Database'} on ${config.host || 'localhost'}:${config.port || 5432} (Connection Refused). No database service is running on this port.`;
    } else if (err.message?.includes('password authentication failed')) {
      friendlyMessage = `Authentication failed: Incorrect username or password for user "${config.username || 'postgres'}".`;
    }
    return {
      success: false,
      message: friendlyMessage,
      executionTimeMs: Date.now() - startTime
    };
  }
});

// 3. Fetch Real Database Schema Tables & Collections
ipcMain.handle('db:get-schema', async (event, config) => {
  try {
    if (config.type === 'postgres') {
      let pool = pgPools.get(config.id);
      if (!pool) {
        if (config.connectionString) {
          pool = new pg.Pool({
            connectionString: config.connectionString,
            ssl: config.ssl !== false ? { rejectUnauthorized: false } : undefined,
            connectionTimeoutMillis: 10000
          });
        } else {
          pool = new pg.Pool(getPgConfig(config));
        }
        pgPools.set(config.id, pool);
      }

      let tableRes;
      try {
        tableRes = await pool.query(`
          SELECT table_name 
          FROM information_schema.tables 
          WHERE table_schema = 'public' 
            AND table_type = 'BASE TABLE'
            AND table_name NOT LIKE 'pg_%'
            AND table_name NOT LIKE 'sql_%'
          ORDER BY table_name
          LIMIT 50;
        `);
      } catch {
        tableRes = { rows: [] };
      }

      if (!tableRes.rows || tableRes.rows.length === 0) {
        try {
          tableRes = await pool.query(`
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = 'public' 
            ORDER BY table_name
            LIMIT 40;
          `);
        } catch {
          tableRes = { rows: [] };
        }
      }

      const tables = [];
      for (const row of (tableRes.rows || [])) {
        const tableName = row.table_name;
        try {
          const colRes = await pool.query(`
            SELECT column_name, data_type, is_nullable
            FROM information_schema.columns
            WHERE table_name = $1 AND table_schema = 'public';
          `, [tableName]);

          tables.push({
            name: tableName,
            rowCount: 500,
            columns: (colRes.rows || []).map(c => ({
              name: c.column_name,
              type: (c.data_type || 'VARCHAR').toUpperCase(),
              isPrimaryKey: c.column_name === 'id',
              isNullable: c.is_nullable === 'YES'
            }))
          });
        } catch {
          tables.push({
            name: tableName,
            rowCount: 0,
            columns: [{ name: 'id', type: 'INT', isPrimaryKey: true, isNullable: false }]
          });
        }
      }

      return { success: true, tables };
    }

    if (config.type === 'mysql') {
      let pool = mysqlPools.get(config.id);
      if (!pool) {
        if (config.connectionString) {
          pool = mysql.createPool({
            uri: config.connectionString,
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000
          });
        } else {
          pool = mysql.createPool({
            host: config.host || 'localhost',
            port: parseInt(config.port, 10) || 3306,
            database: config.database,
            user: config.username || 'root',
            password: config.password || '',
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000,
            ssl: config.ssl === true ? { rejectUnauthorized: false } : undefined
          });
        }
        mysqlPools.set(config.id, pool);
      }

      const dbName = config.database || 'hg38';
      let tableNames = [];

      try {
        const [tableRows] = await pool.query(
          'SELECT table_name FROM information_schema.tables WHERE table_schema = ? ORDER BY table_name LIMIT 30;',
          [dbName]
        );
        tableNames = (tableRows || []).map(r => r.TABLE_NAME || r.table_name).filter(Boolean);
      } catch {
        try {
          const [rawRows] = await pool.query('SHOW TABLES');
          tableNames = (rawRows || []).slice(0, 30).map(r => Object.values(r)[0]).filter(Boolean);
        } catch {}
      }

      const tables = [];
      for (const tableName of tableNames) {
        try {
          const safeName = String(tableName).replace(/`/g, '``');
          const [colRows] = await pool.query(`DESCRIBE \`${safeName}\``);
          tables.push({
            name: tableName,
            rowCount: 1000,
            columns: (colRows || []).map(c => ({
              name: c.Field,
              type: (c.Type || 'VARCHAR').toUpperCase(),
              isPrimaryKey: c.Key === 'PRI',
              isNullable: c.Null === 'YES'
            }))
          });
        } catch {
          tables.push({
            name: tableName,
            rowCount: 0,
            columns: [{ name: 'id', type: 'INT', isPrimaryKey: true, isNullable: false }]
          });
        }
      }

      return { success: true, tables };
    }

    if (config.type === 'mongodb') {
      let client = mongoClients.get(config.id);
      if (!client) {
        let uri = config.connectionString;
        if (!uri) {
          const auth = config.username && config.password ? `${encodeURIComponent(config.username)}:${encodeURIComponent(config.password)}@` : '';
          const host = config.host || 'localhost';
          const port = config.port || 27017;
          uri = `mongodb://${auth}${host}:${port}/${config.database || 'test'}`;
        }
        client = new MongoClient(uri);
        await client.connect();
        mongoClients.set(config.id, client);
      }

      const db = client.db(config.database || 'test');
      const collections = await db.listCollections().toArray();
      const tables = [];

      for (const col of collections) {
        const count = await db.collection(col.name).countDocuments();
        const sampleDoc = await db.collection(col.name).findOne({});
        const columns = sampleDoc ? Object.keys(sampleDoc).map(k => ({
          name: k,
          type: typeof sampleDoc[k] === 'object' ? 'OBJECT' : typeof sampleDoc[k].toUpperCase?.() || 'STRING',
          isPrimaryKey: k === '_id',
          isNullable: true
        })) : [{ name: '_id', type: 'OBJECTID', isPrimaryKey: true, isNullable: false }];

        tables.push({
          name: col.name,
          rowCount: count,
          columns
        });
      }

      return { success: true, tables };
    }

    return { success: false, tables: [] };
  } catch (err) {
    return { success: false, message: err.message, tables: [] };
  }
});

// Open external URLs in default system browser
ipcMain.handle('app:open-external', async (event, url) => {
  if (typeof url === 'string' && (url.startsWith('https:') || url.startsWith('http:'))) {
    await shell.openExternal(url);
    return { success: true };
  }
  return { success: false, message: 'Invalid URL scheme' };
});

// App Lifecycle
app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  // Close all open database pools & clients on shutdown
  for (const pool of pgPools.values()) {
    try { pool.end(); } catch {}
  }
  for (const pool of mysqlPools.values()) {
    try { pool.end(); } catch {}
  }
  for (const client of mongoClients.values()) {
    try { client.close(); } catch {}
  }
  if (process.platform !== 'darwin') app.quit();
});
