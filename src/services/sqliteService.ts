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

  close(dbId: string): void {
    const db = liveDatabases.get(dbId);
    if (db) {
      db.close();
      liveDatabases.delete(dbId);
    }
  }
};
