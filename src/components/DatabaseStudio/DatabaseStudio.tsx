import React, { useState, useEffect, useRef } from 'react';
import { 
  Play, 
  Table, 
  Key, 
  Download, 
  Sparkles, 
  Clock, 
  Check, 
  Layers,
  Database,
  Search,
  Settings,
  FileCode,
  Plus,
  X,
  Terminal,
  Zap,
  Copy,
  AlertTriangle,
  Trash2,
  Lock
} from 'lucide-react';
import { DatabaseConnection, TableSchema, SqlScriptTab } from '../../types';
import { SqliteDropZone } from './SqliteDropZone';
import { AiCopilotModal } from '../AiCopilot/AiCopilotModal';
import { NewConnectionModal } from './NewConnectionModal';
import { DatabaseService } from '../../services/databaseService';
import { SAMPLE_PLAYGROUND_DB, SAMPLE_MONGODB_PLAYGROUND_DB } from '../../data/mockData';

interface DatabaseStudioProps {
  activeDb: DatabaseConnection;
  onRecordHistory: (title: string, subtitle: string, status?: number) => void;
  onDatabaseLoaded?: (db: DatabaseConnection) => void;
  onRenameDatabase?: (dbId: string, newName: string) => void;
  onUpdateDatabase?: (db: DatabaseConnection) => void;
}

interface QueryResult {
  columns: string[];
  rows: any[];
  rowCount: number;
  executionTimeMs: number;
  error?: string;
  queryPlan?: string;
  command?: string;
}

// Redis has no tables/SELECT — the schema browser groups sample keys by TYPE into pseudo-tables
// (table.name is the Redis type, e.g. "hash"), so clicking one should run the command that type
// actually supports, seeded with a real sample key so the click shows real data immediately.
function getDefaultQueryForTable(dbType: string, table: TableSchema): string {
  if (dbType === 'mongodb') {
    return `${table.name}.find({})`;
  }
  if (dbType === 'redis') {
    const sampleKey = table.columns[0]?.name;
    if (!sampleKey) return 'SCAN 0 MATCH * COUNT 25';
    switch (table.name) {
      case 'string': return `GET ${sampleKey}`;
      case 'hash': return `HGETALL ${sampleKey}`;
      case 'list': return `LRANGE ${sampleKey} 0 24`;
      case 'set': return `SMEMBERS ${sampleKey}`;
      case 'zset': return `ZRANGE ${sampleKey} 0 24 WITHSCORES`;
      case 'stream': return `XRANGE ${sampleKey} - +`;
      default: return `TYPE ${sampleKey}`;
    }
  }
  return `SELECT * FROM ${table.name} LIMIT 25;`;
}

function escapeRegExp(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// Inline grid editing writes a real UPDATE/DELETE targeted at a specific table, so it's only safe to
// offer when the result on screen actually IS that table's rows unmodified — not a JOIN, aggregate, or
// hand-edited query that merely happens to share column names.
function isSimpleTableBrowse(dbType: string, lastSql: string, tableName: string): boolean {
  if (!lastSql || !tableName) return false;
  const trimmed = lastSql.trim();
  if (dbType === 'mongodb') {
    return new RegExp(`^${escapeRegExp(tableName)}\\s*\\.\\s*find\\s*\\(`, 'i').test(trimmed);
  }
  const lower = trimmed.toLowerCase().replace(/;+\s*$/, '');
  if (!lower.startsWith('select')) return false;
  if (lower.includes(' join ') || lower.includes('group by') || lower.includes('union')) return false;
  const fromMatch = lower.match(/\bfrom\s+["`]?([a-z0-9_]+)["`]?/i);
  return !!fromMatch && fromMatch[1].toLowerCase() === tableName.toLowerCase();
}

export const DatabaseStudio: React.FC<DatabaseStudioProps> = ({
  activeDb,
  onRecordHistory,
  onDatabaseLoaded,
  onRenameDatabase,
  onUpdateDatabase
}) => {
  const safeDb = activeDb || { id: 'db-empty', name: 'No Connection', type: 'postgres', database: '', isConnected: false, tables: [] };
  const safeTables = Array.isArray(safeDb.tables) ? safeDb.tables : [];

  const [selectedTable, setSelectedTable] = useState<TableSchema | null>(
    safeTables[0] || null
  );
  const [sqlQuery, setSqlQuery] = useState<string>('');
  const [isExecuting, setIsExecuting] = useState(false);
  const [searchFilter, setSearchFilter] = useState('');
  const [isAiModalOpen, setIsAiModalOpen] = useState(false);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);

  const editorRef = useRef<HTMLTextAreaElement>(null);

  // DBeaver-style multiple script tabs state
  const [scriptTabs, setScriptTabs] = useState<SqlScriptTab[]>(() => {
    try {
      const saved = localStorage.getItem(`bapu_db_scripts_${safeDb.id}`);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      }
    } catch {}
    return [{
      id: `tab-${safeDb.id}-1`,
      databaseId: safeDb.id,
      name: safeDb.type === 'mongodb' ? 'Script 1.mongodb.js' : 'Script 1.sql',
      query: ''
    }];
  });

  const [activeTabId, setActiveTabId] = useState<string>(() => scriptTabs[0]?.id || 'tab-1');
  const [editingTabId, setEditingTabId] = useState<string | null>(null);
  const [editingTabName, setEditingTabName] = useState<string>('');
  const [selectionToast, setSelectionToast] = useState<string | null>(null);
  const [activeResultView, setActiveResultView] = useState<'grid' | 'console'>('grid');
  const [copiedPlan, setCopiedPlan] = useState(false);
  const [copiedSql, setCopiedSql] = useState(false);
  const [lastExecutedSql, setLastExecutedSql] = useState<string>('');

  // Data Grid inline editing state (edit cell / add row / delete rows)
  const [editingCell, setEditingCell] = useState<{ rowKey: string; col: string } | null>(null);
  const [editCellValue, setEditCellValue] = useState('');
  const [isSavingCell, setIsSavingCell] = useState(false);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Set<string>>(new Set());
  const [isAddRowOpen, setIsAddRowOpen] = useState(false);
  const [newRowValues, setNewRowValues] = useState<Record<string, string>>({});
  const [newRowJson, setNewRowJson] = useState('{\n  \n}');
  const [rowMutationError, setRowMutationError] = useState<string | null>(null);
  const [isDeletingRows, setIsDeletingRows] = useState(false);

  // Resizable schema sidebar state
  const [dbSidebarWidth, setDbSidebarWidth] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('bapu_db_sidebar_width');
      if (saved) return Math.max(160, Math.min(480, Number(saved)));
    } catch {}
    return 220;
  });
  const [isResizingDb, setIsResizingDb] = useState(false);

  useEffect(() => {
    if (!isResizingDb) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = Math.max(160, Math.min(480, e.clientX - 260));
      setDbSidebarWidth(newWidth);
      try {
        localStorage.setItem('bapu_db_sidebar_width', String(newWidth));
      } catch {}
    };

    const handleMouseUp = () => {
      setIsResizingDb(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingDb]);

  // Synchronize script tabs when active database connection changes
  useEffect(() => {
    try {
      const saved = localStorage.getItem(`bapu_db_scripts_${safeDb.id}`);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) {
          setScriptTabs(parsed);
          setActiveTabId(parsed[0].id);
          setSqlQuery(parsed[0].query || '');
          return;
        }
      }
    } catch {}

    const initialTab: SqlScriptTab = {
      id: `tab-${safeDb.id}-${Date.now()}`,
      databaseId: safeDb.id,
      name: safeDb.type === 'mongodb' ? 'Script 1.mongodb.js' : 'Script 1.sql',
      query: ''
    };
    setScriptTabs([initialTab]);
    setActiveTabId(initialTab.id);
    setSqlQuery('');
  }, [safeDb.id]);

  // Query results state
  const [queryResult, setQueryResult] = useState<QueryResult>({
    columns: [],
    rows: [],
    rowCount: 0,
    executionTimeMs: 0
  });

  // Auto-sync initial table preview when active database changes
  useEffect(() => {
    const firstTable = safeTables.find(t => !t.name.startsWith('pg_') && !t.name.startsWith('sql_')) || safeTables[0];
    if (firstTable && safeDb.id !== 'db-empty') {
      setSelectedTable(firstTable);
      const query = getDefaultQueryForTable(safeDb.type, firstTable);

      // Set initial query on active tab
      setSqlQuery(query);
      setScriptTabs(prev => {
        const updated = prev.map(t => t.id === activeTabId ? { ...t, query } : t);
        try { localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated)); } catch {}
        return updated;
      });

      setLastExecutedSql(query);
      DatabaseService.executeQuery(safeDb, query).then(res => {
        if (res.success) {
          setQueryResult({
            columns: res.columns || [],
            rows: res.rows || [],
            rowCount: res.rowCount || 0,
            executionTimeMs: res.executionTimeMs || 0,
            command: (res as any).command,
            queryPlan: (res as any).queryPlan
          });
        } else {
          setQueryResult({
            columns: [],
            rows: [],
            rowCount: 0,
            executionTimeMs: res.executionTimeMs || 0,
            error: res.message
          });
        }
      }).catch(err => {
        setQueryResult({
          columns: [],
          rows: [],
          rowCount: 0,
          executionTimeMs: 0,
          error: err?.message || 'Execution failed'
        });
      });
    } else {
      setSelectedTable(null);
      setSqlQuery('');
      setQueryResult({
        columns: [],
        rows: [],
        rowCount: 0,
        executionTimeMs: 0
      });
    }
  }, [safeDb.id, safeTables.length]);

  // A freshly executed query invalidates any in-progress cell edit / row selection from the previous result.
  useEffect(() => {
    setEditingCell(null);
    setRowMutationError(null);
    setSelectedRowKeys(new Set());
  }, [queryResult]);

  const handleSelectScriptTab = (tabId: string) => {
    const tab = scriptTabs.find(t => t.id === tabId);
    if (!tab) return;
    setActiveTabId(tabId);
    setSqlQuery(tab.query || '');
  };

  const handleAddScriptTab = () => {
    const ext = safeDb.type === 'mongodb' ? 'mongodb.js' : 'sql';
    const newTab: SqlScriptTab = {
      id: `tab-${safeDb.id}-${Date.now()}`,
      databaseId: safeDb.id,
      name: `Script ${scriptTabs.length + 1}.${ext}`,
      query: safeDb.type === 'mongodb' ? '// New MongoDB Query Script\n' : '-- New SQL Query Script\n'
    };
    const updated = [...scriptTabs, newTab];
    setScriptTabs(updated);
    setActiveTabId(newTab.id);
    setSqlQuery(newTab.query);
    try {
      localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated));
    } catch {}
  };

  const handleCloseScriptTab = (tabId: string) => {
    if (scriptTabs.length <= 1) return;
    const remaining = scriptTabs.filter(t => t.id !== tabId);
    setScriptTabs(remaining);
    if (activeTabId === tabId) {
      const nextTab = remaining[0];
      setActiveTabId(nextTab.id);
      setSqlQuery(nextTab.query || '');
    }
    try {
      localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(remaining));
    } catch {}
  };

  const handleRenameScriptTab = (tabId: string, newName: string) => {
    setEditingTabId(null);
    if (!newName.trim()) return;
    const updated = scriptTabs.map(t => t.id === tabId ? { ...t, name: newName.trim() } : t);
    setScriptTabs(updated);
    try {
      localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated));
    } catch {}
  };

  const handleQueryChange = (newVal: string) => {
    setSqlQuery(newVal);
    setScriptTabs(prev => {
      const updated = prev.map(t => t.id === activeTabId ? { ...t, query: newVal } : t);
      try {
        localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated));
      } catch {}
      return updated;
    });
  };

  const handleExecuteSql = async (overrideQuery?: string) => {
    let queryToRun = overrideQuery;

    // If no override was provided, check if user highlighted a specific block in the editor
    if (!queryToRun && editorRef.current) {
      const { selectionStart, selectionEnd, value } = editorRef.current;
      if (selectionStart !== selectionEnd) {
        const selectedText = value.substring(selectionStart, selectionEnd).trim();
        if (selectedText) {
          queryToRun = selectedText;
          setSelectionToast(`⚡ Running selection (${selectedText.length} chars)`);
          setTimeout(() => setSelectionToast(null), 2500);
        }
      }
    }

    if (!queryToRun) {
      queryToRun = sqlQuery;
    }

    if (!queryToRun || !queryToRun.trim()) return;

    setLastExecutedSql(queryToRun);
    setIsExecuting(true);
    const result = await DatabaseService.executeQuery(safeDb, queryToRun);
    setIsExecuting(false);

    if (result.success) {
      setQueryResult({
        columns: result.columns || [],
        rows: result.rows || [],
        rowCount: result.rowCount || 0,
        executionTimeMs: result.executionTimeMs || 0,
        queryPlan: (result as any).queryPlan,
        command: (result as any).command
      });
      onRecordHistory(`SQL: ${queryToRun.substring(0, 30)}...`, `${result.rowCount} rows • ${result.executionTimeMs}ms`);
    } else {
      setQueryResult(prev => ({
        ...prev,
        error: result.message,
        executionTimeMs: result.executionTimeMs || 0
      }));
      onRecordHistory(`SQL Error: ${queryToRun.substring(0, 25)}`, result.message || 'Execution error');
      setActiveResultView('console');
    }
  };

  const handleExplainQuery = () => {
    let queryToExplain = sqlQuery;
    if (editorRef.current) {
      const { selectionStart, selectionEnd, value } = editorRef.current;
      if (selectionStart !== selectionEnd) {
        const selectedText = value.substring(selectionStart, selectionEnd).trim();
        if (selectedText) queryToExplain = selectedText;
      }
    }
    if (!queryToExplain.trim()) return;

    const wrapped = DatabaseService.wrapExplainQuery(safeDb.type, queryToExplain);
    setSqlQuery(wrapped);
    handleQueryChange(wrapped);
    setActiveResultView('console');
    handleExecuteSql(wrapped);
  };

  const handleExportCsv = () => {
    if (!queryResult.rows || queryResult.rows.length === 0) return;
    const header = (queryResult.columns || []).join(',');
    const rows = queryResult.rows.map(r => (queryResult.columns || []).map(col => JSON.stringify(r[col] || '')).join(','));
    const csvContent = 'data:text/csv;charset=utf-8,' + [header, ...rows].join('\n');
    const encodedUri = encodeURI(csvContent);
    const link = document.createElement('a');
    link.setAttribute('href', encodedUri);
    link.setAttribute('download', `${(safeDb.name || 'db').toLowerCase().replace(/\s+/g, '_')}_export.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const filteredRows = (queryResult.rows || []).filter(row => {
    if (!searchFilter) return true;
    return Object.values(row || {}).some(val =>
      String(val ?? '').toLowerCase().includes(searchFilter.toLowerCase())
    );
  });

  // --- Data Grid inline editing eligibility & handlers ---------------------------------------

  // SQLite is always a real, locally-loaded database (never the built-in demo/playground), so it's
  // exempt from the "no host/connectionString" playground heuristic that otherwise flags it.
  const isPlaygroundDb = safeDb.type !== 'sqlite' && Boolean(
    safeDb.isDemoDb ||
    safeDb.id === 'db-playground-analytics' ||
    safeDb.id?.startsWith('db-demo') ||
    safeDb.id === 'db-empty' ||
    (!safeDb.connectionString && !(safeDb as any).host)
  );

  const editablePkColumns: string[] | null = (() => {
    if (safeDb.type === 'redis' || !selectedTable) return null;
    if (safeDb.type === 'mongodb') {
      return queryResult.columns.includes('_id') ? ['_id'] : null;
    }
    const pkCols = (selectedTable.columns || []).filter(c => c.isPrimaryKey).map(c => c.name);
    if (pkCols.length === 0 || !pkCols.every(c => queryResult.columns.includes(c))) return null;
    return pkCols;
  })();

  const editingDisabledReason: string | null = (() => {
    if (queryResult.error || queryResult.columns.length === 0) return null;
    if (safeDb.type === 'redis') return 'Redis keyspace browsing has no row-based table to edit.';
    if (isPlaygroundDb) return 'Read-only sample data — connect a real database to edit rows.';
    if (!selectedTable) return 'Select a table from the sidebar to enable editing.';
    if (!editablePkColumns) {
      return safeDb.type === 'mongodb'
        ? 'No _id column in this result — editing needs it.'
        : 'No primary key detected on this table — editing is disabled to avoid updating the wrong row.';
    }
    if (!isSimpleTableBrowse(safeDb.type, lastExecutedSql, selectedTable.name)) {
      return 'Editing is only available while browsing a single table, not a custom query/join/aggregate.';
    }
    return null;
  })();

  const isGridEditable = queryResult.columns.length > 0 && editingDisabledReason === null;

  const getRowKey = (row: any): string => {
    if (!editablePkColumns) return JSON.stringify(row);
    return JSON.stringify(editablePkColumns.map(c => row[c]));
  };

  const getRowWhere = (row: any): Record<string, any> => {
    const where: Record<string, any> = {};
    (editablePkColumns || []).forEach(c => { where[c] = row[c]; });
    return where;
  };

  const refreshGridAfterMutation = () => {
    if (lastExecutedSql) handleExecuteSql(lastExecutedSql);
  };

  const commitCellEdit = async (row: any) => {
    if (!editingCell || !selectedTable) return;
    const { col } = editingCell;
    const originalValue = row[col];
    const raw = editCellValue;
    if (String(originalValue ?? '') === raw) {
      setEditingCell(null);
      return;
    }
    // An emptied cell clears the value to NULL; otherwise the typed text is sent as-is and the
    // driver/DB coerces it to the column's real type.
    const newValue = raw === '' ? null : raw;

    setIsSavingCell(true);
    setRowMutationError(null);
    const res = await DatabaseService.mutateRow(safeDb, selectedTable.name, 'update', { [col]: newValue }, getRowWhere(row));
    setIsSavingCell(false);
    setEditingCell(null);

    if (!res.success) {
      setRowMutationError(res.message || 'Update failed');
      return;
    }
    onRecordHistory(`Updated row in ${selectedTable.name}`, `SET ${col} = ${raw === '' ? 'NULL' : raw}`);
    refreshGridAfterMutation();
  };

  const handleAddRowSubmit = async () => {
    if (!selectedTable) return;
    setRowMutationError(null);

    let values: Record<string, any> = {};
    if (safeDb.type === 'mongodb') {
      try {
        values = JSON.parse(newRowJson);
      } catch (err: any) {
        setRowMutationError(`Invalid JSON: ${err.message}`);
        return;
      }
    } else {
      (selectedTable.columns || []).forEach(col => {
        const raw = newRowValues[col.name];
        if (raw !== undefined && raw !== '') {
          values[col.name] = raw;
        }
      });
    }

    setIsSavingCell(true);
    const res = await DatabaseService.mutateRow(safeDb, selectedTable.name, 'insert', values);
    setIsSavingCell(false);

    if (!res.success) {
      setRowMutationError(res.message || 'Insert failed');
      return;
    }
    onRecordHistory(`Inserted row into ${selectedTable.name}`, `${Object.keys(values).length} column(s) set`);
    setIsAddRowOpen(false);
    setNewRowValues({});
    setNewRowJson('{\n  \n}');
    refreshGridAfterMutation();
  };

  const toggleRowSelection = (key: string) => {
    setSelectedRowKeys(prev => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  };

  const toggleSelectAllRows = () => {
    if (selectedRowKeys.size === filteredRows.length && filteredRows.length > 0) {
      setSelectedRowKeys(new Set());
    } else {
      setSelectedRowKeys(new Set(filteredRows.map(getRowKey)));
    }
  };

  const handleDeleteSelectedRows = async () => {
    if (!selectedTable || selectedRowKeys.size === 0) return;
    if (!window.confirm(`Delete ${selectedRowKeys.size} row(s) from ${selectedTable.name}? This cannot be undone.`)) return;

    setIsDeletingRows(true);
    setRowMutationError(null);
    const rowsToDelete = filteredRows.filter(row => selectedRowKeys.has(getRowKey(row)));
    let failures = 0;
    for (const row of rowsToDelete) {
      const res = await DatabaseService.mutateRow(safeDb, selectedTable.name, 'delete', {}, getRowWhere(row));
      if (!res.success) failures += 1;
    }
    setIsDeletingRows(false);
    setSelectedRowKeys(new Set());

    if (failures > 0) {
      setRowMutationError(`${failures} of ${rowsToDelete.length} row(s) failed to delete.`);
    } else {
      onRecordHistory(`Deleted ${rowsToDelete.length} row(s) from ${selectedTable.name}`, '');
    }
    refreshGridAfterMutation();
  };

  return (
    <div className="db-layout" style={{ userSelect: isResizingDb ? 'none' : 'auto' }}>
      {/* Left Database Schema Browser */}
      <aside 
        className="db-schema-sidebar"
        style={{ width: `${dbSidebarWidth}px`, flexShrink: 0, borderRight: 'none' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '12px' }}>
          <Database size={15} color="#10b981" style={{ flexShrink: 0 }} />
          <input
            type="text"
            value={safeDb.name || ''}
            onChange={(e) => {
              if (onRenameDatabase) onRenameDatabase(safeDb.id, e.target.value);
            }}
            placeholder="Connection Name"
            className="request-title-input"
            style={{ fontSize: '12px', fontWeight: 700, padding: '2px 6px', width: '100%', minWidth: '100px' }}
            title="Click to rename connection"
          />
          <button
            onClick={() => setIsEditModalOpen(true)}
            className="sidebar-action-btn"
            title="Edit Database Connection Parameters & Credentials"
            style={{ padding: '4px', flexShrink: 0 }}
          >
            <Settings size={13} color="var(--text-muted)" />
          </button>
        </div>

        {/* Local SQLite Drag and Drop Zone — full-size call-to-action only while there's no
            active connection yet; once one exists it collapses to a small link so it stops
            permanently eating space above the real Tables list. */}
        {onDatabaseLoaded && (
          <SqliteDropZone onDatabaseLoaded={onDatabaseLoaded} compact={safeDb.id !== 'db-empty'} />
        )}

        <div className="sidebar-section-header">Tables ({safeTables.length})</div>
        {safeTables.length === 0 ? (
          <div style={{ fontSize: '11px', color: 'var(--text-dim)', padding: '16px 8px', lineHeight: 1.4, textAlign: 'center' }}>
            <p style={{ marginBottom: '10px' }}>
              {safeDb.id === 'db-empty' ? 'No database connection selected.' : 'No tables discovered.'}
            </p>
            {onDatabaseLoaded && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <button
                  onClick={() => onDatabaseLoaded(SAMPLE_PLAYGROUND_DB)}
                  className="btn-secondary"
                  style={{ fontSize: '11px', padding: '6px 10px', color: '#10b981', borderColor: 'rgba(16, 185, 129, 0.3)', width: '100%' }}
                >
                  ⚡ Load SQL Playground
                </button>
                <button
                  onClick={() => onDatabaseLoaded(SAMPLE_MONGODB_PLAYGROUND_DB)}
                  className="btn-secondary"
                  style={{ fontSize: '11px', padding: '6px 10px', color: '#06b6d4', borderColor: 'rgba(6, 182, 212, 0.3)', width: '100%' }}
                >
                  🍃 Load MongoDB Playground
                </button>
              </div>
            )}
          </div>
        ) : (
          safeTables.map(table => (
            <div key={table?.name || Math.random().toString()} style={{ marginBottom: '8px' }}>
              <div
                onClick={() => {
                  setSelectedTable(table);
                  const query = getDefaultQueryForTable(safeDb.type, table);
                  setSqlQuery(query);
                  handleExecuteSql(query);
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: '5px 8px',
                  borderRadius: 'var(--radius-sm)',
                  cursor: 'pointer',
                  background: selectedTable?.name === table.name ? 'var(--bg-card-hover)' : 'transparent',
                  color: selectedTable?.name === table.name ? '#fff' : 'var(--text-muted)'
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <Table size={13} color="#60a5fa" />
                  <span style={{ fontSize: '12px', fontWeight: 500 }}>{table.name}</span>
                </div>
                <span style={{ fontSize: '10px', color: 'var(--text-dim)', fontFamily: 'var(--font-mono)' }}>
                  {(table.rowCount ?? 0).toLocaleString()}
                </span>
              </div>

              {selectedTable?.name === table.name && (
                <div style={{ paddingLeft: '18px', marginTop: '4px' }}>
                  {(table.columns || []).map(col => (
                    <div key={col.name} style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', color: 'var(--text-dim)', padding: '2px 0' }}>
                      {col.isPrimaryKey ? <Key size={10} color="#f59e0b" /> : <div style={{ width: '10px' }} />}
                      <span style={{ color: col.isPrimaryKey ? 'var(--text-main)' : 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>{col.name}</span>
                      <span style={{ fontSize: '9px', color: 'var(--text-dim)' }}>{col.type}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))
        )}
      </aside>

      {/* Vertical Resize Handle between Schema Browser and Main Query Pane */}
      <div
        className={`pane-resizer-vertical ${isResizingDb ? 'resizing' : ''}`}
        onMouseDown={() => setIsResizingDb(true)}
        onDoubleClick={() => {
          setDbSidebarWidth(220);
          localStorage.setItem('bapu_db_sidebar_width', '220');
        }}
        title="Drag to resize schema browser • Double-click to reset (220px)"
      />

      {/* Main SQL / Mongo Console & Results Pane */}
      <div className="db-main-pane">
        {/* Top Query Editor Box with DBeaver-Style Multi-Script Tabs */}
        <div className="sql-editor-box">
          {/* Script Tabs Bar */}
          <div style={{
            display: 'flex',
            alignItems: 'center',
            background: 'var(--bg-card)',
            borderBottom: '1px solid var(--border-subtle)',
            padding: '0 8px',
            gap: '4px',
            overflowX: 'auto',
            minHeight: '34px'
          }}>
            {scriptTabs.map(tab => {
              const isActive = tab.id === activeTabId;
              const isEditing = editingTabId === tab.id;
              return (
                <div
                  key={tab.id}
                  onClick={() => handleSelectScriptTab(tab.id)}
                  onDoubleClick={() => {
                    setEditingTabId(tab.id);
                    setEditingTabName(tab.name);
                  }}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    padding: '6px 10px',
                    borderBottom: isActive ? '2px solid #10b981' : '2px solid transparent',
                    background: isActive ? 'var(--bg-input)' : 'transparent',
                    color: isActive ? '#fff' : 'var(--text-dim)',
                    fontSize: '11px',
                    cursor: 'pointer',
                    userSelect: 'none',
                    whiteSpace: 'nowrap',
                    borderRadius: 'var(--radius-sm) var(--radius-sm) 0 0',
                    transition: 'all 0.15s ease'
                  }}
                  title="Click to switch script • Double-click to rename"
                >
                  <FileCode size={12} color={isActive ? '#10b981' : 'var(--text-dim)'} />
                  {isEditing ? (
                    <input
                      type="text"
                      value={editingTabName}
                      autoFocus
                      onChange={(e) => setEditingTabName(e.target.value)}
                      onBlur={() => handleRenameScriptTab(tab.id, editingTabName)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') handleRenameScriptTab(tab.id, editingTabName);
                        if (e.key === 'Escape') setEditingTabId(null);
                      }}
                      onClick={(e) => e.stopPropagation()}
                      style={{
                        background: 'var(--bg-main)',
                        border: '1px solid var(--border-subtle)',
                        color: 'var(--text-main)',
                        fontSize: '11px',
                        padding: '1px 4px',
                        borderRadius: '2px',
                        outline: 'none'
                      }}
                    />
                  ) : (
                    <span>{tab.name}</span>
                  )}

                  {scriptTabs.length > 1 && (
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleCloseScriptTab(tab.id);
                      }}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--text-dim)',
                        cursor: 'pointer',
                        padding: '1px',
                        display: 'flex',
                        alignItems: 'center',
                        borderRadius: '2px'
                      }}
                      title="Close script tab"
                    >
                      <X size={11} />
                    </button>
                  )}
                </div>
              );
            })}

            <button
              type="button"
              onClick={handleAddScriptTab}
              style={{
                background: 'none',
                border: 'none',
                color: 'var(--text-dim)',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                padding: '4px 8px',
                fontSize: '11px',
                borderRadius: 'var(--radius-sm)'
              }}
              title="Add new script scratchpad"
            >
              <Plus size={12} />
              <span>New Script</span>
            </button>

            {selectionToast && (
              <div style={{
                marginLeft: 'auto',
                fontSize: '10px',
                color: '#10b981',
                background: 'rgba(16, 185, 129, 0.12)',
                border: '1px solid rgba(16, 185, 129, 0.3)',
                borderRadius: 'var(--radius-sm)',
                padding: '2px 8px'
              }}>
                {selectionToast}
              </div>
            )}
          </div>

          <textarea
            ref={editorRef}
            value={sqlQuery}
            onChange={(e) => handleQueryChange(e.target.value)}
            onKeyDown={(e) => {
              if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                e.preventDefault();
                handleExecuteSql();
              } else if (e.key === 'F5') {
                e.preventDefault();
                handleExecuteSql();
              }
            }}
            className="code-textarea"
            spellCheck={false}
            placeholder={activeDb.type === 'mongodb'
              ? 'Write MongoDB JSON query or MQL (e.g. { "status": "active" } or users.find({})) • Highlight query & press Ctrl+Enter to run selection'
              : activeDb.type === 'redis'
              ? 'Write a Redis command (e.g. GET mykey, HGETALL user:1, KEYS *) • Highlight query & press Ctrl+Enter to run selection'
              : 'Write SQL query here... (e.g. SELECT * FROM table;) • Highlight query & press Ctrl+Enter to run selection'}
          />

          <div className="sql-actions-bar" style={{ flexWrap: 'wrap', gap: '6px' }}>
            <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap', alignItems: 'center' }}>
              {selectedTable && (
                safeDb.type === 'mongodb' ? (
                  <>
                    <button 
                      onClick={() => {
                        const q = `${selectedTable.name}.find({})`;
                        handleQueryChange(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                    >
                      Find All ({selectedTable.name})
                    </button>
                    <button 
                      onClick={() => {
                        const cols = selectedTable.columns || [];
                        const catCol = cols.find(c => /status|role|type|category|plan/i.test(c.name))?.name || 'status';
                        const q = `${selectedTable.name}.aggregate([\n  { $group: { _id: "$${catCol}", count: { $sum: 1 } } },\n  { $sort: { count: -1 } }\n])`;
                        handleQueryChange(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                    >
                      📊 Group & Count
                    </button>
                  </>
                ) : (
                  <>
                    <button 
                      onClick={() => {
                        const cols = selectedTable.columns || [];
                        const dateCol = cols.find(c => /date|time|created|updated|at$/i.test(c.name))?.name || (cols.find(c => c.isPrimaryKey)?.name || 'id');
                        const q = `SELECT * FROM ${selectedTable.name} ORDER BY ${dateCol} DESC LIMIT 25;`;
                        handleQueryChange(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Fetch recent rows ordered by date/ID"
                    >
                      ⏱️ Recent
                    </button>
                    <button 
                      onClick={() => {
                        const cols = selectedTable.columns || [];
                        const catCol = cols.find(c => /status|role|type|category|plan|state/i.test(c.name))?.name || (cols.length > 1 ? cols[1].name : 'id');
                        const q = `SELECT ${catCol}, COUNT(*) as total FROM ${selectedTable.name} GROUP BY ${catCol} ORDER BY total DESC;`;
                        handleQueryChange(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Group and count records"
                    >
                      📊 Group & Count
                    </button>
                    <button 
                      onClick={() => {
                        const cols = selectedTable.columns || [];
                        const nullCol = cols.find(c => !c.isPrimaryKey)?.name || 'id';
                        const q = `SELECT * FROM ${selectedTable.name} WHERE ${nullCol} IS NULL;`;
                        handleQueryChange(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Find records with NULL values"
                    >
                      🔍 Find Nulls
                    </button>
                    <button 
                      onClick={() => {
                        const cols = selectedTable.columns || [];
                        const colStr = cols.length > 0 ? cols.map(c => c.name).join(', ') : '*';
                        const q = `SELECT ${colStr} FROM ${selectedTable.name} LIMIT 50;`;
                        handleQueryChange(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Select all explicit columns"
                    >
                      📋 Column List
                    </button>
                  </>
                )
              )}
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginLeft: 'auto' }}>
              <button
                onClick={() => setIsAiModalOpen(true)}
                className="btn-secondary"
                style={{ borderColor: 'rgba(16, 185, 129, 0.3)', color: '#6ee7b7' }}
                title="Generate SQL from Natural Language"
              >
                <Sparkles size={13} color="#10b981" />
                <span>AI SQL Copilot</span>
              </button>

              <button
                type="button"
                onClick={handleExplainQuery}
                disabled={isExecuting}
                className="btn-secondary"
                style={{ borderColor: 'rgba(245, 158, 11, 0.4)', color: '#fbbf24', padding: '6px 12px', fontSize: '12px' }}
                title="Explain Query Execution Plan (Costs, Scans, Buffers)"
              >
                <Zap size={13} color="#f59e0b" />
                <span>Explain Plan</span>
              </button>

              <button 
                onClick={() => handleExecuteSql()}
                disabled={isExecuting}
                className="btn-send"
                style={{ padding: '6px 14px', fontSize: '12px' }}
                title="Run Query or Selection (Ctrl+Enter / F5)"
              >
                <Play size={13} />
                <span>{isExecuting ? 'Running...' : 'Run Query (F5)'}</span>
              </button>
            </div>
          </div>
        </div>

        {/* Results Toolbar with Dual View Tabs (Data Grid vs Execution Console) */}
        <div className="panel-tab-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <div className="panel-tabs">
              <button
                className={`subtab-btn ${activeResultView === 'grid' ? 'active' : ''}`}
                onClick={() => setActiveResultView('grid')}
                style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
              >
                <Table size={13} />
                <span>Data Grid ({queryResult.rowCount})</span>
              </button>
              <button
                className={`subtab-btn ${activeResultView === 'console' ? 'active' : ''}`}
                onClick={() => setActiveResultView('console')}
                style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
              >
                <Terminal size={13} />
                <span>Execution Console</span>
                {queryResult.error ? (
                  <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: '#ef4444' }} />
                ) : queryResult.rowCount > 0 ? (
                  <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: '#10b981' }} />
                ) : null}
              </button>
            </div>

            <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', marginLeft: '6px' }}>
              {queryResult.rowCount} rows • {queryResult.executionTimeMs}ms
            </span>
          </div>

          {activeResultView === 'grid' && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', background: 'var(--bg-input)', padding: '2px 8px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                <Search size={11} color="var(--text-dim)" />
                <input
                  type="text"
                  value={searchFilter}
                  onChange={(e) => setSearchFilter(e.target.value)}
                  placeholder="Filter table rows..."
                  style={{ background: 'transparent', border: 'none', color: 'var(--text-main)', fontSize: '11px', outline: 'none', width: '120px' }}
                />
              </div>

              <button onClick={handleExportCsv} className="btn-secondary" style={{ fontSize: '11px', padding: '3px 8px' }}>
                <Download size={12} />
                <span>Export CSV</span>
              </button>

              {isGridEditable ? (
                <>
                  <button
                    onClick={() => { setNewRowValues({}); setNewRowJson('{\n  \n}'); setRowMutationError(null); setIsAddRowOpen(true); }}
                    className="btn-secondary"
                    style={{ fontSize: '11px', padding: '3px 8px', borderColor: 'rgba(16, 185, 129, 0.4)', color: '#6ee7b7' }}
                  >
                    <Plus size={12} />
                    <span>Add Row</span>
                  </button>

                  {selectedRowKeys.size > 0 && (
                    <button
                      onClick={handleDeleteSelectedRows}
                      disabled={isDeletingRows}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px', borderColor: 'rgba(239, 68, 68, 0.4)', color: '#fca5a5' }}
                    >
                      <Trash2 size={12} />
                      <span>{isDeletingRows ? 'Deleting...' : `Delete Selected (${selectedRowKeys.size})`}</span>
                    </button>
                  )}
                </>
              ) : editingDisabledReason && queryResult.columns.length > 0 ? (
                <span
                  title={editingDisabledReason}
                  style={{ display: 'flex', alignItems: 'center', gap: '5px', fontSize: '10px', color: 'var(--text-dim)', padding: '3px 8px' }}
                >
                  <Lock size={11} />
                  <span>Read-only</span>
                </span>
              ) : null}
            </div>
          )}
        </div>

        {/* Row Mutation Error (edit / add / delete) */}
        {rowMutationError && (
          <div style={{
            margin: '8px 12px 0',
            padding: '8px 12px',
            background: 'rgba(239, 68, 68, 0.1)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            borderRadius: 'var(--radius-sm)',
            fontSize: '11px',
            color: '#f87171',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '8px'
          }}>
            <span>{rowMutationError}</span>
            <button onClick={() => setRowMutationError(null)} className="btn-secondary" style={{ fontSize: '10px', padding: '2px 6px' }}>
              Dismiss
            </button>
          </div>
        )}

        {/* Error Diagnostic Alert */}
        {queryResult.error && (
          <div style={{
            margin: '8px 12px',
            padding: '10px 14px',
            background: 'rgba(239, 68, 68, 0.1)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            borderRadius: 'var(--radius-sm)',
            fontSize: '12px',
            color: '#f87171',
            fontFamily: 'var(--font-mono)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between'
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <AlertTriangle size={15} color="#ef4444" />
              <span>{queryResult.error}</span>
            </div>
            <button
              onClick={() => setActiveResultView('console')}
              className="btn-secondary"
              style={{ fontSize: '10px', padding: '2px 8px', borderColor: 'rgba(239, 68, 68, 0.4)', color: '#fca5a5' }}
            >
              View Console Details
            </button>
          </div>
        )}

        {/* Dual Content View: Data Grid or Execution Console */}
        {activeResultView === 'grid' ? (
          <div className="data-grid-container">
            {queryResult.columns.length === 0 ? (
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-dim)', gap: '8px', padding: '40px' }}>
                <Database size={24} style={{ opacity: 0.3 }} />
                <span style={{ fontSize: '12px' }}>
                  {safeDb.id === 'db-empty'
                    ? 'No database connection connected. Add a connection in the sidebar or drop a SQLite file.'
                    : 'Write a query and press Run Query (F5) to view results'}
                </span>
              </div>
            ) : (
              <table className="data-grid-table">
                <thead>
                  <tr>
                    {isGridEditable && (
                      <th style={{ width: '28px' }}>
                        <input
                          type="checkbox"
                          checked={selectedRowKeys.size > 0 && selectedRowKeys.size === filteredRows.length}
                          onChange={toggleSelectAllRows}
                          title="Select all rows"
                        />
                      </th>
                    )}
                    {queryResult.columns.map(col => {
                      const isPk = (editablePkColumns || []).includes(col);
                      return (
                        <th key={col}>
                          {col}
                          {isPk && <Key size={10} style={{ marginLeft: '4px', opacity: 0.6, verticalAlign: 'middle' }} />}
                        </th>
                      );
                    })}
                  </tr>
                </thead>
                <tbody>
                  {filteredRows.map((row, rIdx) => {
                    const rowKey = getRowKey(row);
                    return (
                      <tr key={rIdx}>
                        {isGridEditable && (
                          <td>
                            <input
                              type="checkbox"
                              checked={selectedRowKeys.has(rowKey)}
                              onChange={() => toggleRowSelection(rowKey)}
                            />
                          </td>
                        )}
                        {queryResult.columns.map(col => {
                          const isPk = (editablePkColumns || []).includes(col);
                          const cellEditable = isGridEditable && !isPk;
                          const isEditingThisCell = editingCell && editingCell.rowKey === rowKey && editingCell.col === col;

                          if (isEditingThisCell) {
                            return (
                              <td key={col} style={{ padding: 0 }}>
                                <input
                                  autoFocus
                                  value={editCellValue}
                                  disabled={isSavingCell}
                                  onChange={(e) => setEditCellValue(e.target.value)}
                                  onBlur={() => commitCellEdit(row)}
                                  onKeyDown={(e) => {
                                    if (e.key === 'Enter') { e.preventDefault(); commitCellEdit(row); }
                                    if (e.key === 'Escape') { e.preventDefault(); setEditingCell(null); }
                                  }}
                                  style={{
                                    width: '100%',
                                    boxSizing: 'border-box',
                                    background: 'var(--bg-input)',
                                    border: '1px solid var(--border-focus)',
                                    color: 'var(--text-main)',
                                    fontSize: 'inherit',
                                    fontFamily: 'inherit',
                                    padding: '6px 10px',
                                    outline: 'none'
                                  }}
                                />
                              </td>
                            );
                          }

                          return (
                            <td
                              key={col}
                              onDoubleClick={() => {
                                if (!cellEditable) return;
                                setEditingCell({ rowKey, col });
                                setEditCellValue(row[col] === null || row[col] === undefined ? '' : (typeof row[col] === 'object' ? JSON.stringify(row[col]) : String(row[col])));
                              }}
                              title={cellEditable ? 'Double-click to edit' : isPk ? 'Primary key — not editable' : undefined}
                              style={cellEditable ? { cursor: 'text' } : undefined}
                            >
                              {row[col] === null || row[col] === undefined ? (
                                <span style={{ opacity: 0.5, fontStyle: 'italic' }}>NULL</span>
                              ) : typeof row[col] === 'object' ? (
                                JSON.stringify(row[col])
                              ) : (
                                String(row[col])
                              )}
                            </td>
                          );
                        })}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
          </div>
        ) : (
          /* Execution Console & Server Output View */
          <div style={{ flex: 1, overflowY: 'auto', padding: '16px', background: '#080c13', display: 'flex', flexDirection: 'column', gap: '16px' }}>
            {/* Metadata Stats Grid */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(140px, 1fr))', gap: '10px' }}>
              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Status</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: queryResult.error ? '#ef4444' : '#10b981', marginTop: '4px' }}>
                  {queryResult.error ? 'FAILED' : 'SUCCESS (200 OK)'}
                </div>
              </div>

              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Execution Time</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: '#60a5fa', marginTop: '4px', fontFamily: 'var(--font-mono)' }}>
                  {queryResult.executionTimeMs} ms
                </div>
              </div>

              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Rows Affected</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: '#f59e0b', marginTop: '4px', fontFamily: 'var(--font-mono)' }}>
                  {queryResult.rowCount.toLocaleString()}
                </div>
              </div>

              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Driver / Command</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: '#a78bfa', marginTop: '4px', fontFamily: 'var(--font-mono)' }}>
                  {safeDb.type.toUpperCase()} • {queryResult.command || (safeDb.type === 'mongodb' ? 'MQL' : 'QUERY')}
                </div>
              </div>
            </div>

            {/* Execution Plan Output (if available) */}
            {queryResult.queryPlan && (
              <div style={{ background: 'var(--bg-card)', border: '1px solid rgba(245, 158, 11, 0.3)', borderRadius: 'var(--radius-md)', overflow: 'hidden' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(245, 158, 11, 0.08)', padding: '8px 12px', borderBottom: '1px solid rgba(245, 158, 11, 0.2)' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', fontWeight: 600, color: '#fbbf24' }}>
                    <Zap size={13} />
                    <span>Query Execution Plan (Explain Analyzer)</span>
                  </div>
                  <button
                    onClick={() => {
                      navigator.clipboard.writeText(queryResult.queryPlan || '');
                      setCopiedPlan(true);
                      setTimeout(() => setCopiedPlan(false), 2000);
                    }}
                    className="btn-secondary"
                    style={{ fontSize: '10px', padding: '2px 8px' }}
                  >
                    {copiedPlan ? <Check size={11} color="#10b981" /> : <Copy size={11} />}
                    <span>{copiedPlan ? 'Copied' : 'Copy Plan'}</span>
                  </button>
                </div>
                <pre style={{ margin: 0, padding: '14px', fontFamily: 'var(--font-mono)', fontSize: '11px', color: '#e2e8f0', lineHeight: 1.5, overflowX: 'auto', background: '#05080e' }}>
                  {queryResult.queryPlan}
                </pre>
              </div>
            )}

            {/* Executed Statement Block */}
            <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', overflow: 'hidden' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--bg-panel-header)', padding: '8px 12px', borderBottom: '1px solid var(--border-subtle)' }}>
                <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-muted)' }}>Executed SQL / Command</span>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(lastExecutedSql || sqlQuery);
                    setCopiedSql(true);
                    setTimeout(() => setCopiedSql(false), 2000);
                  }}
                  className="btn-secondary"
                  style={{ fontSize: '10px', padding: '2px 8px' }}
                >
                  {copiedSql ? <Check size={11} color="#10b981" /> : <Copy size={11} />}
                  <span>{copiedSql ? 'Copied' : 'Copy Query'}</span>
                </button>
              </div>
              <pre style={{ margin: 0, padding: '14px', fontFamily: 'var(--font-mono)', fontSize: '11px', color: '#60a5fa', lineHeight: 1.5, overflowX: 'auto', background: '#05080e' }}>
                {lastExecutedSql || sqlQuery || '-- No query executed yet'}
              </pre>
            </div>
          </div>
        )}
      </div>

      {/* AI Copilot Modal for SQL */}
      <AiCopilotModal
        isOpen={isAiModalOpen}
        onClose={() => setIsAiModalOpen(false)}
        activeDb={safeDb}
        selectedTable={selectedTable}
        onApplySql={(generatedSql) => {
          setSqlQuery(generatedSql);
        }}
      />

      {/* Edit Connection Configuration Modal */}
      <NewConnectionModal
        isOpen={isEditModalOpen}
        initialConnection={safeDb}
        onClose={() => setIsEditModalOpen(false)}
        onConnect={(updatedDb) => {
          if (onUpdateDatabase) onUpdateDatabase(updatedDb);
          setIsEditModalOpen(false);
        }}
      />

      {/* Add Row Modal — a plain column form for SQL engines, a JSON document editor for MongoDB */}
      {isAddRowOpen && selectedTable && (
        <div className="modal-overlay" onClick={() => setIsAddRowOpen(false)}>
          <div className="modal-content" style={{ maxWidth: '520px' }} onClick={(e) => e.stopPropagation()}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '16px' }}>
              <h3 style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-main)' }}>
                Add Row — {selectedTable.name}
              </h3>
              <button onClick={() => setIsAddRowOpen(false)} className="btn-secondary" style={{ padding: '4px' }}>
                <X size={14} />
              </button>
            </div>

            {rowMutationError && (
              <div style={{ marginBottom: '12px', padding: '8px 12px', background: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.3)', borderRadius: 'var(--radius-sm)', fontSize: '11px', color: '#f87171' }}>
                {rowMutationError}
              </div>
            )}

            {safeDb.type === 'mongodb' ? (
              <div>
                <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '6px' }}>
                  New document (JSON) — _id is generated automatically
                </label>
                <textarea
                  value={newRowJson}
                  onChange={(e) => setNewRowJson(e.target.value)}
                  rows={10}
                  style={{
                    width: '100%',
                    boxSizing: 'border-box',
                    background: 'var(--bg-input)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 'var(--radius-sm)',
                    color: 'var(--text-main)',
                    fontFamily: 'var(--font-mono)',
                    fontSize: '12px',
                    padding: '10px',
                    resize: 'vertical'
                  }}
                />
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', maxHeight: '360px', overflowY: 'auto' }}>
                {(selectedTable.columns || []).map(col => (
                  <div key={col.name}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                      {col.name}
                      <span style={{ opacity: 0.6 }}>({col.type}{col.isPrimaryKey ? ', PK' : ''}{col.isNullable ? '' : ', required'})</span>
                    </label>
                    <input
                      type="text"
                      value={newRowValues[col.name] ?? ''}
                      onChange={(e) => setNewRowValues(prev => ({ ...prev, [col.name]: e.target.value }))}
                      placeholder={col.isPrimaryKey ? 'Leave blank to auto-generate, if supported' : col.isNullable ? 'NULL' : ''}
                      style={{
                        width: '100%',
                        boxSizing: 'border-box',
                        background: 'var(--bg-input)',
                        border: '1px solid var(--border-subtle)',
                        borderRadius: 'var(--radius-sm)',
                        color: 'var(--text-main)',
                        fontSize: '12px',
                        padding: '7px 10px',
                        outline: 'none'
                      }}
                    />
                  </div>
                ))}
              </div>
            )}

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px', marginTop: '18px' }}>
              <button onClick={() => setIsAddRowOpen(false)} className="btn-secondary">
                Cancel
              </button>
              <button onClick={handleAddRowSubmit} disabled={isSavingCell} className="btn-send" style={{ padding: '7px 16px' }}>
                {isSavingCell ? 'Inserting...' : 'Insert Row'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
