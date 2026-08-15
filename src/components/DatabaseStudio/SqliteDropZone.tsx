import React, { useState } from 'react';
import { UploadCloud, FileSpreadsheet, CheckCircle2, Database } from 'lucide-react';
import { DatabaseConnection } from '../../types';

interface SqliteDropZoneProps {
  onDatabaseLoaded: (db: DatabaseConnection) => void;
}

export const SqliteDropZone: React.FC<SqliteDropZoneProps> = ({ onDatabaseLoaded }) => {
  const [isDragging, setIsDragging] = useState(false);
  const [loadedFileName, setLoadedFileName] = useState<string | null>(null);

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);

    const files = e.dataTransfer.files;
    if (files.length > 0) {
      processFile(files[0]);
    }
  };

  const handleFileInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      processFile(files[0]);
    }
  };

  const processFile = (file: File) => {
    setLoadedFileName(file.name);

    // Mock parsing SQLite file metadata (or reading via Tauri rusqlite / wasm)
    const newDb: DatabaseConnection = {
      id: `sqlite-${Date.now()}`,
      name: `Local SQLite (${file.name})`,
      type: 'sqlite',
      database: file.name,
      isConnected: true,
      tables: [
        {
          name: 'app_settings',
          rowCount: 48,
          columns: [
            { name: 'key', type: 'TEXT', isPrimaryKey: true, isNullable: false },
            { name: 'value', type: 'TEXT', isPrimaryKey: false, isNullable: true },
            { name: 'updated_at', type: 'DATETIME', isPrimaryKey: false, isNullable: false }
          ]
        },
        {
          name: 'local_cache',
          rowCount: 1250,
          columns: [
            { name: 'id', type: 'INTEGER', isPrimaryKey: true, isNullable: false },
            { name: 'endpoint', type: 'TEXT', isPrimaryKey: false, isNullable: false },
            { name: 'payload_json', type: 'TEXT', isPrimaryKey: false, isNullable: true },
            { name: 'expires_at', type: 'DATETIME', isPrimaryKey: false, isNullable: false }
          ]
        },
        {
          name: 'audit_events',
          rowCount: 382,
          columns: [
            { name: 'id', type: 'INTEGER', isPrimaryKey: true, isNullable: false },
            { name: 'action', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
            { name: 'user_agent', type: 'TEXT', isPrimaryKey: false, isNullable: true },
            { name: 'timestamp', type: 'DATETIME', isPrimaryKey: false, isNullable: false }
          ]
        }
      ]
    };

    setTimeout(() => {
      onDatabaseLoaded(newDb);
    }, 400);
  };

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{
        border: `2px dashed ${isDragging ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
        background: isDragging ? 'rgba(59, 130, 246, 0.08)' : 'rgba(15, 21, 34, 0.4)',
        borderRadius: 'var(--radius-lg)',
        padding: '24px 16px',
        textAlign: 'center',
        margin: '12px 0',
        transition: 'all 0.15s ease',
        cursor: 'pointer',
        position: 'relative'
      }}
    >
      <input
        type="file"
        accept=".sqlite,.db,.sqlite3"
        onChange={handleFileInput}
        style={{
          position: 'absolute',
          inset: 0,
          opacity: 0,
          cursor: 'pointer',
          width: '100%',
          height: '100%'
        }}
      />

      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px', pointerEvents: 'none' }}>
        {loadedFileName ? (
          <>
            <CheckCircle2 size={24} color="#10b981" />
            <span style={{ fontSize: '12px', fontWeight: 600, color: '#10b981' }}>
              Loaded {loadedFileName}!
            </span>
          </>
        ) : (
          <>
            <UploadCloud size={24} color={isDragging ? '#3b82f6' : 'var(--text-dim)'} />
            <div style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-main)' }}>
              Drag & Drop SQLite File (<code>.sqlite</code> / <code>.db</code>)
            </div>
            <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
              Zero setup required. Inspect tables and run queries instantly.
            </span>
          </>
        )}
      </div>
    </div>
  );
};
