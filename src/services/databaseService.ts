import { DatabaseConnection, TableSchema } from '../types';
import { SqliteService } from './sqliteService';

export interface SqlQueryResult {
  success: boolean;
  columns: string[];
  rows: any[];
  rowCount: number;
  executionTimeMs: number;
  message?: string;
  command?: string;
  queryPlan?: string;
}

export interface ConnectionTestResult {
  success: boolean;
  latencyMs?: number;
  message: string;
}

export interface RowMutation {
  op: 'insert' | 'update' | 'delete';
  values?: Record<string, any>;
  where?: Record<string, any>;
}

// Helper to access the secure context bridge exposed by preload.cjs
function getBridge(): any {
  if (typeof window !== 'undefined' && (window as any).bapuBridge) {
    return (window as any).bapuBridge;
  }
  return null;
}

export const DatabaseService = {
  wrapExplainQuery(dbType: string, sql: string): string {
    const trimmed = sql.trim().replace(/;+$/, '');
    if (trimmed.toUpperCase().startsWith('EXPLAIN')) return sql;

    if (dbType === 'postgres') {
      return `EXPLAIN (ANALYZE, COSTS, VERBOSE, BUFFERS)\n${trimmed};`;
    }
    if (dbType === 'mysql') {
      return `EXPLAIN FORMAT=JSON\n${trimmed};`;
    }
    if (dbType === 'sqlite') {
      return `EXPLAIN QUERY PLAN\n${trimmed};`;
    }
    if (dbType === 'mongodb') {
      if (trimmed.includes('.find(') || trimmed.includes('.aggregate(')) {
        return `${trimmed}.explain("executionStats")`;
      }
      return trimmed;
    }
    if (dbType === 'redis') {
      // Redis has no query planner/execution-plan concept — nothing to wrap.
      return trimmed;
    }
    return `EXPLAIN ${trimmed};`;
  },

  async testConnection(config: any): Promise<ConnectionTestResult> {
    const isPlayground = Boolean(
      config.isDemoDb || 
      config.id === 'db-playground-analytics' || 
      config.id?.startsWith('db-demo') ||
      (!config.connectionString && !(config as any).host)
    );

    // If testing the built-in demo/playground connection, confirm readiness immediately
    if (isPlayground) {
      await new Promise((resolve) => setTimeout(resolve, 80));
      return {
        success: true,
        latencyMs: 1,
        message: `⚡ Built-in Playground Database is active and ready (1ms)`
      };
    }

    const bridge = getBridge();
    if (bridge) {
      try {
        const res = await bridge.dbTestConnection(config);
        return res;
      } catch (err: any) {
        return { success: false, message: err.message || 'Connection failed' };
      }
    }

    // Browser simulation fallback
    await new Promise((resolve) => setTimeout(resolve, 300));
    return {
      success: true,
      latencyMs: 14,
      message: `Simulated connection to ${config.type?.toUpperCase()} successful (14ms)`
    };
  },

  async executeQuery(db: DatabaseConnection, sql: string): Promise<SqlQueryResult> {
    // SQLite always runs in-process via sql.js (no server, no IPC bridge involved).
    if (db.type === 'sqlite') {
      return SqliteService.executeQuery(db.id, sql);
    }

    const isPlayground = Boolean(
      db.isDemoDb ||
      db.id === 'db-playground-analytics' ||
      db.id?.startsWith('db-demo') ||
      db.id === 'db-empty' ||
      (!db.connectionString && !(db as any).host)
    );
    const bridge = getBridge();

    // If it's a real user-added database with host or connectionString, ALWAYS execute natively via Electron IPC driver
    if (bridge && !isPlayground) {
      try {
        const res = await bridge.dbQuery({ config: db, sql });
        return res;
      } catch (err: any) {
        return {
          success: false,
          columns: [],
          rows: [],
          rowCount: 0,
          executionTimeMs: 0,
          message: `Connection Error: ${err.message || 'Could not reach database server'}. Ensure the database is reachable.`
        };
      }
    }

    // If SQL is empty or whitespace, return clean empty result
    if (!sql || !sql.trim()) {
      return {
        success: true,
        columns: [],
        rows: [],
        rowCount: 0,
        executionTimeMs: 0
      };
    }

    // If no active connection configured
    if (db.id === 'db-empty' || (!db.database && (!db.tables || db.tables.length === 0) && !db.connectionString)) {
      return {
        success: false,
        columns: [],
        rows: [],
        rowCount: 0,
        executionTimeMs: 0,
        message: 'No active database connection. Please add or select a database connection first.'
      };
    }

    // Built-in Demo & In-Memory SQL Execution Engine
    await new Promise((resolve) => setTimeout(resolve, 80));
    
    // Clean SQL comments (-- comment or /* comment */)
    const cleanSql = sql
      .replace(/--.*$/gm, '')
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .trim();
    const lower = cleanSql.toLowerCase();

    // 1. SELECT queries
    if (lower.startsWith('select') || lower.includes('select ')) {
      // Helper to project requested columns from raw table objects
      const projectColumns = (rawRows: any[], requestedColumnsStr: string, allPossibleColumns: string[]): { columns: string[]; rows: any[] } => {
        const trimmed = requestedColumnsStr.trim();
        if (trimmed === '*' || !trimmed) {
          return { columns: allPossibleColumns, rows: rawRows };
        }

        const requestedCols = trimmed
          .split(',')
          .map(c => c.trim().replace(/^[`"']|[`"']$/g, '').toLowerCase())
          .filter(Boolean);

        const matchedCols = allPossibleColumns.filter(c => requestedCols.includes(c.toLowerCase()));
        const finalCols = matchedCols.length > 0 ? matchedCols : allPossibleColumns;

        const projectedRows = rawRows.map(row => {
          const newRow: Record<string, any> = {};
          finalCols.forEach(col => {
            newRow[col] = row[col];
          });
          return newRow;
        });

        return { columns: finalCols, rows: projectedRows };
      };

      // Extract requested column projection
      const selectMatch = cleanSql.match(/select\s+([\s\S]+?)\s+from\s+(\w+)/i);
      const requestedColsStr = selectMatch ? selectMatch[1].trim() : '*';
      const limitMatch = cleanSql.match(/limit\s+(\d+)/i);
      const limitVal = limitMatch ? parseInt(limitMatch[1], 10) : null;

      // EXPLAIN query plan simulation
      if (lower.startsWith('explain')) {
        const planLines = [
          'Seq Scan on users  (cost=0.00..38.25 rows=1420 width=128) (actual time=0.012..0.450 rows=1420 loops=1)',
          '  Filter: (status = \'ACTIVE\'::text)',
          '  Rows Removed by Filter: 85',
          '  Buffers: shared hit=28',
          'Planning Time: 0.085 ms',
          'Execution Time: 0.512 ms'
        ];
        return {
          success: true,
          columns: ['QUERY PLAN'],
          rows: planLines.map(line => ({ 'QUERY PLAN': line })),
          rowCount: planLines.length,
          executionTimeMs: 9,
          command: 'EXPLAIN',
          queryPlan: planLines.join('\n')
        };
      }

      // JOIN queries between users and accounts
      if (lower.includes('join')) {
        let rows = [
          { user_id: 'u_101', user_name: 'Alex Rivera', email: 'alex.rivera@acme.dev', account_id: 'acc_881', balance_usd: '$2,450.00', status: 'ACTIVE' },
          { user_id: 'u_103', user_name: 'Elena Rostova', email: 'elena.rostova@cloudscale.net', account_id: 'acc_882', balance_usd: '$890.00', status: 'ACTIVE' },
          { user_id: 'u_105', user_name: 'Priya Sharma', email: 'priya.sharma@hyperloop.io', account_id: 'acc_883', balance_usd: '$142.00', status: 'ACTIVE' }
        ];
        if (limitVal !== null) rows = rows.slice(0, limitVal);
        const projected = projectColumns(rows, requestedColsStr, ['user_id', 'user_name', 'email', 'account_id', 'balance_usd', 'status']);
        return {
          success: true,
          columns: projected.columns,
          rows: projected.rows,
          rowCount: projected.rows.length,
          executionTimeMs: 18
        };
      }

      if (lower.includes('count') && lower.includes('group by')) {
        return {
          success: true,
          columns: ['role', 'user_count'],
          rows: [
            { role: 'developer', user_count: 1420 },
            { role: 'admin', user_count: 480 },
            { role: 'billing', user_count: 125 }
          ],
          rowCount: 3,
          executionTimeMs: 12
        };
      }

      if (lower.startsWith('select count(') || lower.startsWith('select count (*)')) {
        return {
          success: true,
          columns: ['total_count'],
          rows: [{ total_count: 1420 }],
          rowCount: 1,
          executionTimeMs: 8
        };
      }

      if (lower.includes('workspaces') || lower.includes('storage')) {
        let rows = [
          { id: 'ws_03', name: 'Mobile API Backend', plan_tier: 'enterprise', storage_mb: 12400, created_at: '2026-02-14' },
          { id: 'ws_01', name: 'Acme Production Infrastructure', plan_tier: 'enterprise', storage_mb: 4500, created_at: '2026-01-15' },
          { id: 'ws_02', name: 'Stripe Sandbox Vault', plan_tier: 'pro', storage_mb: 850, created_at: '2026-02-01' },
          { id: 'ws_04', name: 'Testing Sandbox', plan_tier: 'free', storage_mb: 120, created_at: '2026-02-20' }
        ];

        if (lower.includes('order by') && lower.includes('desc')) {
          rows.sort((a, b) => b.storage_mb - a.storage_mb);
        }

        if (lower.includes('enterprise')) {
          rows = rows.filter(r => r.plan_tier === 'enterprise');
        } else if (lower.includes('pro')) {
          rows = rows.filter(r => r.plan_tier === 'pro');
        }

        if (limitVal !== null) rows = rows.slice(0, limitVal);
        const projected = projectColumns(rows, requestedColsStr, ['id', 'name', 'plan_tier', 'storage_mb', 'created_at']);

        return {
          success: true,
          columns: projected.columns,
          rows: projected.rows,
          rowCount: projected.rows.length,
          executionTimeMs: 14
        };
      }

      if (lower.includes('accounts') || lower.includes('balance')) {
        let rows = [
          { id: 'acc_881', user_id: 'u_101', balance_cents: 245000, currency: 'USD', status: 'ACTIVE' },
          { id: 'acc_882', user_id: 'u_103', balance_cents: 89000, currency: 'USD', status: 'ACTIVE' },
          { id: 'acc_883', user_id: 'u_105', balance_cents: 14200, currency: 'EUR', status: 'ACTIVE' }
        ];
        if (limitVal !== null) rows = rows.slice(0, limitVal);
        const projected = projectColumns(rows, requestedColsStr, ['id', 'user_id', 'balance_cents', 'currency', 'status']);
        return {
          success: true,
          columns: projected.columns,
          rows: projected.rows,
          rowCount: projected.rows.length,
          executionTimeMs: 15
        };
      }

      // Default users table sample
      let rows = [
        { id: 'u_101', email: 'alex.rivera@acme.dev', name: 'Alex Rivera', role: 'admin', status: 'ACTIVE', created_at: '2026-02-10 14:22:01' },
        { id: 'u_102', email: 'sarah.connor@cyberdyne.io', name: 'Sarah Connor', role: 'developer', status: 'ACTIVE', created_at: '2026-02-11 09:15:30' },
        { id: 'u_103', email: 'elena.rostova@cloudscale.net', name: 'Elena Rostova', role: 'owner', status: 'ACTIVE', created_at: '2026-02-12 18:40:12' },
        { id: 'u_104', email: 'david.kim@fintech-ai.com', name: 'David Kim', role: 'developer', status: 'INACTIVE', created_at: '2026-02-13 11:05:44' },
        { id: 'u_105', email: 'priya.sharma@hyperloop.io', name: 'Priya Sharma', role: 'billing', status: 'ACTIVE', created_at: '2026-02-14 16:30:19' }
      ];

      if (lower.includes("role = 'admin'") || (lower.includes('admin') && lower.includes('where'))) {
        rows = rows.filter(r => r.role === 'admin');
      } else if (lower.includes('is null')) {
        rows = [];
      }

      if (lower.includes('order by') && lower.includes('desc')) {
        rows.sort((a, b) => b.created_at.localeCompare(a.created_at));
      }

      if (limitVal !== null) rows = rows.slice(0, limitVal);
      const projected = projectColumns(rows, requestedColsStr, ['id', 'email', 'name', 'role', 'status', 'created_at']);

      return {
        success: true,
        columns: projected.columns,
        rows: projected.rows,
        rowCount: projected.rows.length,
        executionTimeMs: 16
      };
    }

    // 2. MongoDB Document queries (JSON, find, or aggregate)
    if (db.type === 'mongodb' || cleanSql.startsWith('{') || lower.includes('.find(') || lower.includes('.aggregate(') || lower.includes('.explain(')) {
      const isOrders = lower.includes('order');
      const isAggregate = lower.includes('aggregate') || lower.includes('$group');

      if (lower.includes('.explain(')) {
        const planJson = JSON.stringify({
          queryPlanner: {
            plannerVersion: 1,
            namespace: `${db.database || 'store_inventory'}.${isOrders ? 'orders' : 'products'}`,
            winningPlan: {
              stage: 'COLLSCAN',
              direction: 'forward'
            }
          },
          executionStats: {
            executionSuccess: true,
            nReturned: isOrders ? 5 : 6,
            executionTimeMillis: 3,
            totalDocsExamined: isOrders ? 5 : 6
          }
        }, null, 2);
        return {
          success: true,
          columns: ['explain_output'],
          rows: [{ explain_output: planJson }],
          rowCount: 1,
          executionTimeMs: 8,
          command: 'EXPLAIN',
          queryPlan: planJson
        };
      }

      if (isAggregate) {
        if (isOrders) {
          return {
            success: true,
            columns: ['_id', 'count', 'total_sales'],
            rows: [
              { _id: 'completed', count: 112, total_sales: 14850.50 },
              { _id: 'processing', count: 24, total_sales: 3240.00 },
              { _id: 'refunded', count: 9, total_sales: 780.00 }
            ],
            rowCount: 3,
            executionTimeMs: 18
          };
        }
        return {
          success: true,
          columns: ['_id', 'count'],
          rows: [
            { _id: 'hardware', count: 142 },
            { _id: 'software', count: 98 },
            { _id: 'cloud', count: 80 }
          ],
          rowCount: 3,
          executionTimeMs: 18
        };
      }

      if (isOrders) {
        const orderDocs = [
          { _id: '66a1b2c3d4e5f001', customer_email: 'alex.rivera@example.com', total: 149.99, status: 'completed', items_count: 3, created_at: '2026-08-20T10:15:00Z' },
          { _id: '66a1b2c3d4e5f002', customer_email: 'sarah.connor@example.com', total: 29.50, status: 'completed', items_count: 1, created_at: '2026-08-21T14:22:00Z' },
          { _id: '66a1b2c3d4e5f003', customer_email: 'elena.rostova@example.com', total: 499.00, status: 'processing', items_count: 5, created_at: '2026-08-22T08:05:00Z' },
          { _id: '66a1b2c3d4e5f004', customer_email: 'david.kim@example.com', total: 12.00, status: 'completed', items_count: 1, created_at: '2026-08-22T19:40:00Z' },
          { _id: '66a1b2c3d4e5f005', customer_email: 'priya.sharma@example.com', total: 85.00, status: 'refunded', items_count: 2, created_at: '2026-08-23T06:12:00Z' }
        ];
        return {
          success: true,
          columns: ['_id', 'customer_email', 'total', 'status', 'items_count', 'created_at'],
          rows: orderDocs,
          rowCount: orderDocs.length,
          executionTimeMs: 14
        };
      }

      const productDocs = [
        { _id: '65cb7891a123f001', name: 'Mechanical RGB Keyboard', sku: 'SKU-KB-RGB', price: 129.99, inStock: true, category: 'hardware', tags: ['gaming', 'usb-c'] },
        { _id: '65cb7891a123f002', name: 'Studio Pro Wireless Mouse', sku: 'SKU-MS-PRO', price: 79.50, inStock: true, category: 'hardware', tags: ['wireless', 'ergonomic'] },
        { _id: '65cb7891a123f003', name: 'Bapu Developer License (Annual)', sku: 'SKU-BAPU-DEV', price: 99.00, inStock: true, category: 'software', tags: ['subscription', 'api'] },
        { _id: '65cb7891a123f004', name: 'Cloud Secrets Vault (100GB)', sku: 'SKU-VAULT-100', price: 15.00, inStock: true, category: 'cloud', tags: ['storage', 'encryption'] },
        { _id: '65cb7891a123f005', name: '4K Ultra-Wide Monitor 34"', sku: 'SKU-MON-4K34', price: 549.00, inStock: false, category: 'hardware', tags: ['display', 'hdr'] },
        { _id: '65cb7891a123f006', name: 'USB-C Multiport Hub 8-in-1', sku: 'SKU-HUB-8IN1', price: 45.00, inStock: true, category: 'hardware', tags: ['accessories', '4k'] }
      ];

      return {
        success: true,
        columns: ['_id', 'name', 'sku', 'price', 'inStock', 'category', 'tags'],
        rows: productDocs,
        rowCount: productDocs.length,
        executionTimeMs: 14
      };
    }

    return {
      success: true,
      columns: ['status', 'message'],
      rows: [{ status: 'OK', message: 'Command executed successfully. 1 row affected.' }],
      rowCount: 1,
      executionTimeMs: 22
    };
  },

  // DBeaver-style "N pending changes, then Save/Revert" — applies every staged edit/insert/delete as
  // one all-or-nothing batch for Postgres/MySQL/SQLite (a real transaction), or MongoDB when its
  // server supports multi-document transactions (a replica set, which includes every Atlas cluster —
  // a standalone/local `mongod` does not). When it can't be atomic, the response says so via
  // `atomic: false` rather than the UI silently treating a partial apply as a clean save.
  async mutateBatch(
    db: DatabaseConnection,
    table: string,
    mutations: RowMutation[]
  ): Promise<{ success: boolean; atomic: boolean; results?: Array<{ op: string; rowsAffected: number }>; message?: string }> {
    if (mutations.length === 0) {
      return { success: true, atomic: true, results: [] };
    }

    if (db.type === 'sqlite') {
      return SqliteService.mutateBatch(db.id, table, mutations);
    }

    const isPlayground = Boolean(
      db.isDemoDb ||
      db.id === 'db-playground-analytics' ||
      db.id?.startsWith('db-demo') ||
      db.id === 'db-empty' ||
      (!db.connectionString && !(db as any).host)
    );
    if (isPlayground) {
      return { success: false, atomic: true, message: 'This is read-only sample data — connect a real database to edit rows.' };
    }

    const bridge = getBridge();
    if (!bridge?.dbMutateBatch) {
      return { success: false, atomic: true, message: 'Row editing requires the desktop app (no direct database access in a plain browser).' };
    }

    try {
      return await bridge.dbMutateBatch({ config: db, table, mutations });
    } catch (err: any) {
      return { success: false, atomic: true, message: err.message || 'Batch mutation failed' };
    }
  },

  async disconnect(id: string): Promise<void> {
    if (SqliteService.isLoaded(id)) {
      SqliteService.close(id);
      return;
    }
    const bridge = getBridge();
    if (bridge?.dbDisconnect) {
      try {
        await bridge.dbDisconnect(id);
      } catch {}
    }
  },

  async fetchSchema(db: DatabaseConnection): Promise<{ success: boolean; tables: TableSchema[]; message?: string }> {
    if (db.type === 'sqlite') {
      return { success: true, tables: SqliteService.getSchema(db.id) };
    }

    const bridge = getBridge();
    if (bridge) {
      try {
        const res = await bridge.dbGetSchema(db);
        if (res.success && res.tables?.length > 0) {
          return res;
        }
      } catch {}
    }

    return {
      success: true,
      tables: db.tables || []
    };
  }
};
