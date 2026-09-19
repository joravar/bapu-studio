import initSqlJs, { Database, SqlJsStatic } from 'sql.js';
// Vite's `?url` suffix resolves to the built asset's final URL (copied into dist automatically),
// which is what sql.js needs to fetch its WASM binary in the browser/renderer.
// eslint-disable-next-line import/no-unresolved
import sqlWasmUrl from 'sql.js/dist/sql-wasm.wasm?url';
import { ColumnDefinition, TableSchema } from '../types';
import type { SqlQueryResult } from './databaseService';

let sqlJsPromise: Promise<SqlJsStatic> | null = null;

function getSqlJs(): Promise<SqlJsStatic> {
  if (!sqlJsPromise) {
    sqlJsPromise = initSqlJs({ locateFile: () => sqlWasmUrl });
  }
  return sqlJsPromise;
}

// Live sql.js Database instances, keyed by DatabaseConnection.id. This is a runtime-only, in-memory
// registry (not persisted) — the underlying WASM object can't be serialized into localStorage, so a
// SQLite connection only lasts for the current app session and needs the file re-dropped after restart.
const liveDatabases = new Map<string, Database>();

function quoteIdent(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

// sql.js's exec() returns [] for BOTH a write statement and a read query matching zero rows, so the
// caller has to tell those apart itself to avoid reporting a genuinely empty SELECT as a fake "OK".
function isReadQuery(sql: string): boolean {
  const trimmed = sql.trim().replace(/^(--[^\n]*\n|\/\*[\s\S]*?\*\/)\s*/g, '').toUpperCase();
  return /^(SELECT|PRAGMA|EXPLAIN|WITH)\b/.test(trimmed);
}

// Shared by mutateRow (single, immediate) and mutateBatch (several, transactional).
function buildMutationSql(table: string, op: 'insert' | 'update' | 'delete', values: Record<string, any>, where: Record<string, any>): { sql: string; params: any[] } {
  if (op === 'insert') {
    const cols = Object.keys(values);
    const params = cols.map(c => values[c]);
    const sql = `INSERT INTO ${quoteIdent(table)} (${cols.map(quoteIdent).join(', ')}) VALUES (${cols.map(() => '?').join(', ')})`;
    return { sql, params };
  }
  if (op === 'update') {
    const setCols = Object.keys(values);
    const whereCols = Object.keys(where);
    const params = [...setCols.map(c => values[c]), ...whereCols.filter(c => where[c] !== null).map(c => where[c])];
    const setClause = setCols.map(c => `${quoteIdent(c)} = ?`).join(', ');
    const whereClause = whereCols.map(c => where[c] === null ? `${quoteIdent(c)} IS NULL` : `${quoteIdent(c)} = ?`).join(' AND ');
    return { sql: `UPDATE ${quoteIdent(table)} SET ${setClause} WHERE ${whereClause}`, params };
  }
  const whereCols = Object.keys(where);
  const params = whereCols.filter(c => where[c] !== null).map(c => where[c]);
  const whereClause = whereCols.map(c => where[c] === null ? `${quoteIdent(c)} IS NULL` : `${quoteIdent(c)} = ?`).join(' AND ');
  return { sql: `DELETE FROM ${quoteIdent(table)} WHERE ${whereClause}`, params };
}

function extractSchema(db: Database): TableSchema[] {
  const tables: TableSchema[] = [];
  const tableList = db.exec(`SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name;`);
  const tableNames = tableList[0] ? tableList[0].values.map(v => String(v[0])) : [];

  for (const tableName of tableNames) {
    let columns: ColumnDefinition[] = [];
    try {
      const colInfo = db.exec(`PRAGMA table_info(${quoteIdent(tableName)});`);
      if (colInfo[0]) {
        // PRAGMA table_info columns: cid, name, type, notnull, dflt_value, pk
        columns = colInfo[0].values.map(row => ({
          name: String(row[1]),
          type: (row[2] ? String(row[2]) : 'TEXT').toUpperCase(),
          isPrimaryKey: Number(row[5]) > 0,
          isNullable: Number(row[3]) === 0
        }));
      }
    } catch {}

    let rowCount = 0;
    try {
      const countRes = db.exec(`SELECT COUNT(*) FROM ${quoteIdent(tableName)};`);
      rowCount = countRes[0] ? Number(countRes[0].values[0][0]) : 0;
    } catch {}

    tables.push({ name: tableName, rowCount, columns });
  }

  return tables;
}

export const SqliteService = {
  isLoaded(dbId: string): boolean {
    return liveDatabases.has(dbId);
  },

  async loadFromFile(file: File): Promise<{ id: string; tables: TableSchema[] }> {
    const SQL = await getSqlJs();
    const buffer = await file.arrayBuffer();
    const db = new SQL.Database(new Uint8Array(buffer));

    // Validate this is actually a readable SQLite file before committing to it.
    try {
      db.exec(`SELECT name FROM sqlite_master LIMIT 1;`);
    } catch (err: any) {
      db.close();
      throw new Error(`Not a valid SQLite database file: ${err.message}`);
    }

    const id = `sqlite-${Date.now()}`;
    liveDatabases.set(id, db);
    return { id, tables: extractSchema(db) };
  },

  getSchema(dbId: string): TableSchema[] {
    const db = liveDatabases.get(dbId);
    if (!db) return [];
    return extractSchema(db);
  },

  executeQuery(dbId: string, sql: string): SqlQueryResult {
    const startTime = performance.now();
    const db = liveDatabases.get(dbId);
    if (!db) {
      return {
        success: false,
        columns: [],
        rows: [],
        rowCount: 0,
        executionTimeMs: 0,
        message: 'This SQLite database is not loaded in the current session — drag & drop the file again to reconnect.'
      };
    }

    try {
      const results = db.exec(sql);
      const executionTimeMs = Math.round(performance.now() - startTime);

      if (results.length === 0) {
        // sql.js's exec() returns [] both for a write statement AND for a read query that legitimately
        // matched zero rows — those need different, honest responses: a real "0 rows" result for the
        // latter (with real column names for the grid), not a fabricated "OK, N row(s) affected".
        if (isReadQuery(sql)) {
          return { success: true, columns: [], rows: [], rowCount: 0, executionTimeMs };
        }
        const rowsModified = db.getRowsModified();
        return {
          success: true,
          columns: ['status', 'message'],
          rows: [{ status: 'OK', message: `Command executed successfully. ${rowsModified} row(s) affected.` }],
          rowCount: rowsModified,
          executionTimeMs,
          command: sql.trim().split(/\s+/)[0]?.toUpperCase()
        };
      }

      const { columns, values } = results[0];
      const rows = values.map(row => {
        const obj: Record<string, any> = {};
        columns.forEach((col, i) => { obj[col] = row[i]; });
        return obj;
      });

      return { success: true, columns, rows, rowCount: rows.length, executionTimeMs };
    } catch (err: any) {
      return {
        success: false,
        columns: [],
        rows: [],
        rowCount: 0,
        executionTimeMs: Math.round(performance.now() - startTime),
        message: err.message || 'SQLite query execution error'
      };
    }
  },

  // DBeaver-style "pending changes, then Save/Revert" batch — a real SQLite transaction, so a failure
  // partway through rolls back everything already applied in this batch rather than leaving a partial edit.
  mutateBatch(
    dbId: string,
    table: string,
    mutations: Array<{ op: 'insert' | 'update' | 'delete'; values?: Record<string, any>; where?: Record<string, any> }>
  ): { success: boolean; atomic: boolean; results?: Array<{ op: string; rowsAffected: number }>; message?: string } {
    const db = liveDatabases.get(dbId);
    if (!db) {
      return { success: false, atomic: true, message: 'This SQLite database is not loaded in the current session — drag & drop the file again to reconnect.' };
    }
    if (mutations.length === 0) {
      return { success: true, atomic: true, results: [] };
    }

    db.exec('BEGIN');
    try {
      const results = mutations.map(m => {
        const { sql, params } = buildMutationSql(table, m.op, m.values || {}, m.where || {});
        db.run(sql, params);
        return { op: m.op, rowsAffected: db.getRowsModified() };
      });
      db.exec('COMMIT');
      return { success: true, atomic: true, results };
    } catch (err: any) {
      try { db.exec('ROLLBACK'); } catch {}
      return { success: false, atomic: true, message: err.message || 'SQLite batch mutation error' };
    }
  },

  close(dbId: string): void {
    const db = liveDatabases.get(dbId);
    if (db) {
      db.close();
      liveDatabases.delete(dbId);
    }
  }
};
