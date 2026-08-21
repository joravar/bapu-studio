import { DatabaseConnection, TableSchema } from '../types';

export interface SqlQueryResult {
  success: boolean;
  columns: string[];
  rows: any[];
  rowCount: number;
  executionTimeMs: number;
  message?: string;
}

export interface ConnectionTestResult {
  success: boolean;
  latencyMs?: number;
  message: string;
}

// Helper to access the secure context bridge exposed by preload.cjs
function getBridge(): any {
  if (typeof window !== 'undefined' && (window as any).bapuBridge) {
    return (window as any).bapuBridge;
  }
  return null;
}

export const DatabaseService = {
  async testConnection(config: any): Promise<ConnectionTestResult> {
    const isPlayground = Boolean(
      config.id?.includes('demo') || 
      config.name?.toLowerCase().includes('demo') || 
      config.name?.toLowerCase().includes('playground') || 
      config.name?.toLowerCase().includes('sample') ||
      (!config.password && !config.connectionString && (config.host === 'localhost' || !config.host) && (config.database === 'saas_production_db' || config.database === 'nexus_core_db' || config.database === 'main'))
    );

    // If testing the built-in demo/playground connection, confirm readiness immediately
    if (isPlayground) {
      await new Promise((resolve) => setTimeout(resolve, 80));
      return {
        success: true,
        latencyMs: 1,
        message: `⚡ Built-in ${config.type ? config.type.toUpperCase() : 'PostgreSQL'} Playground Database is active and ready (1ms)`
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
    const hasRealCredentials = Boolean(db.connectionString || ((db as any).host && !(db as any).host.includes('localhost') && (db as any).password));
    const isDemoDb = !hasRealCredentials || db.id.includes('demo') || db.id === 'db-main' || db.id === 'db-cache' || db.id === 'db-1' || db.id === 'db-2' || db.name.toLowerCase().includes('demo') || db.name.toLowerCase().includes('sample');
    const bridge = getBridge();

    // If it's a real user-added database with host credentials, run through native IPC driver
    if (bridge && !isDemoDb && ((db as any).host || db.connectionString)) {
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
          message: `Connection Error: ${err.message || 'Could not reach database server'}. Ensure the database is running on the specified host & port.`
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

    // 2. MongoDB Document queries (JSON or find)
    if (db.type === 'mongodb' || cleanSql.startsWith('{') || lower.includes('.find(')) {
      const docs = [
        { _id: '65cb7891a123f001', name: 'Product Catalog', sku: 'SKU-PRO-99', price: 99.00, inStock: true, tags: ['hardware', 'gadgets'] },
        { _id: '65cb7891a123f002', name: 'Developer License', sku: 'SKU-SOFT-12', price: 12.00, inStock: true, tags: ['software', 'subscription'] },
        { _id: '65cb7891a123f003', name: 'Cloud Vault Addon', sku: 'SKU-CLOUD-05', price: 5.00, inStock: true, tags: ['cloud', 'storage'] }
      ];
      return {
        success: true,
        columns: ['_id', 'name', 'sku', 'price', 'inStock', 'tags'],
        rows: docs,
        rowCount: docs.length,
        executionTimeMs: 19
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

  async fetchSchema(db: DatabaseConnection): Promise<{ success: boolean; tables: TableSchema[]; message?: string }> {
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
