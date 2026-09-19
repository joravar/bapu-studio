import React, { useState } from 'react';
import { UploadCloud, CheckCircle2, AlertCircle } from 'lucide-react';
import { DatabaseConnection } from '../../types';
import { SqliteService } from '../../services/sqliteService';

interface SqliteDropZoneProps {
  onDatabaseLoaded: (db: DatabaseConnection) => void;
  // Once a connection is already active, the full drag-and-drop box just eats sidebar space above
  // the real Tables list — collapse it to a small "load another file" link instead.
  compact?: boolean;
}

export const SqliteDropZone: React.FC<SqliteDropZoneProps> = ({ onDatabaseLoaded, compact = false }) => {
  const [isDragging, setIsDragging] = useState(false);
  const [loadedFileName, setLoadedFileName] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

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

  const processFile = async (file: File) => {
    setError(null);
    setIsLoading(true);
    try {
      const { id, tables } = await SqliteService.loadFromFile(file);
      setLoadedFileName(file.name);

      const newDb: DatabaseConnection = {
        id,
        name: `Local SQLite (${file.name})`,
        type: 'sqlite',
        database: file.name,
        isConnected: true,
        tables
      };

      onDatabaseLoaded(newDb);
    } catch (err: any) {
      setError(err.message || 'Failed to open this file as a SQLite database.');
      setLoadedFileName(null);
    } finally {
      setIsLoading(false);
    }
  };

  const fileInput = (
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
  );

  if (compact) {
    return (
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        title="Drag & drop a .sqlite/.db file here, or click to browse"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          border: `1px dashed ${error ? '#ef4444' : isDragging ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
          background: isDragging ? 'rgba(59, 130, 246, 0.08)' : 'transparent',
          borderRadius: 'var(--radius-sm)',
          padding: '5px 8px',
          margin: '8px 0',
          cursor: 'pointer',
          position: 'relative',
          fontSize: '11px'
        }}
      >
        {fileInput}
        {error ? (
          <>
            <AlertCircle size={12} color="#ef4444" style={{ flexShrink: 0 }} />
            <span style={{ color: '#ef4444', fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{error}</span>
          </>
        ) : loadedFileName ? (
          <>
            <CheckCircle2 size={12} color="#10b981" style={{ flexShrink: 0 }} />
            <span style={{ color: '#10b981', fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>Loaded {loadedFileName}</span>
          </>
        ) : (
          <>
            <UploadCloud size={12} color={isDragging ? '#3b82f6' : 'var(--text-dim)'} style={{ flexShrink: 0 }} />
            <span style={{ color: 'var(--text-dim)' }}>{isLoading ? 'Reading database...' : 'Load a SQLite file...'}</span>
          </>
        )}
      </div>
    );
  }

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{
        border: `2px dashed ${error ? '#ef4444' : isDragging ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
        background: isDragging ? 'rgba(59, 130, 246, 0.08)' : 'var(--bg-input)',
        borderRadius: 'var(--radius-lg)',
        padding: '24px 16px',
        textAlign: 'center',
        margin: '12px 0',
        transition: 'all 0.15s ease',
        cursor: 'pointer',
        position: 'relative'
      }}
    >
      {fileInput}

      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px', pointerEvents: 'none' }}>
        {error ? (
          <>
            <AlertCircle size={24} color="#ef4444" />
            <span style={{ fontSize: '12px', fontWeight: 600, color: '#ef4444' }}>
              {error}
            </span>
          </>
        ) : loadedFileName ? (
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
              {isLoading ? 'Reading database...' : (<>Drag & Drop SQLite File (<code>.sqlite</code> / <code>.db</code>)</>)}
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
