import React, { useState } from 'react';
import { 
  History, 
  Trash2, 
  Search, 
  Play, 
  Copy, 
  Check, 
  ArrowUpRight, 
  Database, 
  Globe, 
  AlertCircle, 
  CheckCircle2, 
  Clock, 
  Sparkles,
  Filter
} from 'lucide-react';
import { HistoryItem } from '../../types';

interface HistoryStudioProps {
  history: HistoryItem[];
  onClearHistory: () => void;
  onDeleteHistoryItem: (id: string) => void;
  onReplayItem: (item: HistoryItem) => void;
}

type FilterType = 'all' | 'api' | 'sql' | 'errors';

export const HistoryStudio: React.FC<HistoryStudioProps> = ({
  history,
  onClearHistory,
  onDeleteHistoryItem,
  onReplayItem
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<FilterType>('all');
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const handleCopy = (id: string, text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  const filteredHistory = history.filter(item => {
    // Category filter
    if (filterType === 'api' && item.type !== 'api') return false;
    if (filterType === 'sql' && item.type !== 'sql' && !item.title.startsWith('SQL:')) return false;
    if (filterType === 'errors') {
      const isErrorStatus = item.status && (item.status >= 400 || item.title.includes('Error') || item.subtitle.includes('Error'));
      if (!isErrorStatus) return false;
    }

    // Search query
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    return (
      item.title.toLowerCase().includes(q) ||
      item.subtitle.toLowerCase().includes(q) ||
      item.timestamp.toLowerCase().includes(q)
    );
  });

  const apiCount = history.filter(h => h.type === 'api' || (!h.title.startsWith('SQL:') && !h.title.startsWith('Connected:'))).length;
  const sqlCount = history.filter(h => h.type === 'sql' || h.title.startsWith('SQL:')).length;
  const errorCount = history.filter(h => (h.status && h.status >= 400) || h.title.includes('Error') || h.subtitle.includes('Error')).length;

  return (
    <div style={{ flex: 1, overflowY: 'auto', padding: '24px', background: 'var(--bg-main)' }}>
      <div style={{ maxWidth: '960px', margin: '0 auto' }}>
        {/* Header Bar */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '20px' }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '4px' }}>
              <History size={20} color="#06b6d4" />
              <h1 style={{ fontSize: '20px', fontWeight: 700, color: '#fff', margin: 0 }}>
                Activity & Execution History
              </h1>
            </div>
            <p style={{ fontSize: '12px', color: 'var(--text-muted)', margin: 0 }}>
              Live audit trail of all API requests, SQL queries, and connection events in this workspace session.
            </p>
          </div>

          {history.length > 0 && (
            <button
              onClick={() => {
                if (confirm('Are you sure you want to clear all execution history?')) {
                  onClearHistory();
                }
              }}
              className="btn-secondary"
              style={{ color: '#ef4444', borderColor: 'rgba(239, 68, 68, 0.3)', padding: '6px 12px', fontSize: '12px' }}
            >
              <Trash2 size={13} />
              <span>Clear History ({history.length})</span>
            </button>
          )}
        </div>

        {/* Metrics Summary Strip */}
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(4, 1fr)',
          gap: '12px',
          marginBottom: '20px'
        }}>
          <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '12px 14px' }}>
            <div style={{ fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>TOTAL RECORDED</div>
            <div style={{ fontSize: '18px', fontWeight: 700, color: '#fff' }}>{history.length}</div>
          </div>
          <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '12px 14px' }}>
            <div style={{ fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>HTTP API CALLS</div>
            <div style={{ fontSize: '18px', fontWeight: 700, color: '#06b6d4' }}>{apiCount}</div>
          </div>
          <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '12px 14px' }}>
            <div style={{ fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>SQL / DB QUERIES</div>
            <div style={{ fontSize: '18px', fontWeight: 700, color: '#10b981' }}>{sqlCount}</div>
          </div>
          <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '12px 14px' }}>
            <div style={{ fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>FAILED / ERRORS</div>
            <div style={{ fontSize: '18px', fontWeight: 700, color: errorCount > 0 ? '#ef4444' : '#64748b' }}>{errorCount}</div>
          </div>
        </div>

        {/* Search & Filter Toolbar */}
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          gap: '12px',
          marginBottom: '16px'
        }}>
          {/* Search Input */}
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            background: 'var(--bg-input)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
            padding: '6px 12px',
            flex: 1,
            maxWidth: '380px'
          }}>
            <Search size={14} color="var(--text-dim)" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search history by query, URL, status..."
              style={{
                background: 'transparent',
                border: 'none',
                color: '#fff',
                fontSize: '12px',
                outline: 'none',
                width: '100%'
              }}
            />
          </div>

          {/* Filter Tabs */}
          <div style={{ display: 'flex', gap: '6px', background: 'var(--bg-input)', padding: '3px', borderRadius: 'var(--radius-md)' }}>
            <button
              onClick={() => setFilterType('all')}
              className={`subtab-btn ${filterType === 'all' ? 'active' : ''}`}
              style={{ padding: '4px 10px', fontSize: '11px' }}
            >
              All ({history.length})
            </button>
            <button
              onClick={() => setFilterType('api')}
              className={`subtab-btn ${filterType === 'api' ? 'active' : ''}`}
              style={{ padding: '4px 10px', fontSize: '11px' }}
            >
              APIs ({apiCount})
            </button>
            <button
              onClick={() => setFilterType('sql')}
              className={`subtab-btn ${filterType === 'sql' ? 'active' : ''}`}
              style={{ padding: '4px 10px', fontSize: '11px' }}
            >
              SQL / DB ({sqlCount})
            </button>
            <button
              onClick={() => setFilterType('errors')}
              className={`subtab-btn ${filterType === 'errors' ? 'active' : ''}`}
              style={{ padding: '4px 10px', fontSize: '11px', color: errorCount > 0 ? '#f87171' : undefined }}
            >
              Errors ({errorCount})
            </button>
          </div>
        </div>

        {/* History List */}
        {filteredHistory.length === 0 ? (
          <div style={{
            padding: '48px 24px',
            textAlign: 'center',
            background: 'var(--bg-card)',
            border: '1px dashed var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
            color: 'var(--text-dim)'
          }}>
            <History size={32} style={{ opacity: 0.3, margin: '0 auto 12px' }} />
            <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-muted)' }}>
              {history.length === 0 ? 'No Execution History Yet' : 'No Matching Activity Found'}
            </div>
            <div style={{ fontSize: '12px', marginTop: '4px' }}>
              {history.length === 0 
                ? 'Send an API request or run a database query to see real-time logs here.' 
                : 'Try adjusting your search terms or filter.'}
            </div>
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            {filteredHistory.map((item) => {
              const isSql = item.type === 'sql' || item.title.startsWith('SQL:');
              const isConnection = item.title.startsWith('Connected:');
              const isError = (item.status && item.status >= 400) || item.title.includes('Error') || item.subtitle.includes('Error');

              return (
                <div
                  key={item.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '10px 14px',
                    background: 'var(--bg-card)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 'var(--radius-md)',
                    transition: 'all 0.15s ease'
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flex: 1, minWidth: 0, marginRight: '16px' }}>
                    {/* Protocol Icon */}
                    <div style={{
                      padding: '8px',
                      borderRadius: 'var(--radius-sm)',
                      background: isSql ? 'rgba(16, 185, 129, 0.1)' : isConnection ? 'rgba(168, 85, 247, 0.1)' : 'rgba(6, 182, 212, 0.1)',
                      color: isSql ? '#10b981' : isConnection ? '#a855f7' : '#06b6d4',
                      flexShrink: 0
                    }}>
                      {isSql ? <Database size={15} /> : isConnection ? <Sparkles size={15} /> : <Globe size={15} />}
                    </div>

                    {/* Content */}
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '2px' }}>
                        <span style={{
                          fontSize: '13px',
                          fontWeight: 600,
                          color: '#fff',
                          fontFamily: isSql ? 'var(--font-mono)' : 'inherit',
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap'
                        }}>
                          {item.title}
                        </span>

                        {isError && (
                          <span style={{ fontSize: '10px', color: '#ef4444', background: 'rgba(239, 68, 68, 0.15)', padding: '1px 6px', borderRadius: '4px', fontWeight: 600 }}>
                            FAILED
                          </span>
                        )}
                      </div>

                      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', fontSize: '11px', color: 'var(--text-dim)' }}>
                        <span style={{ fontFamily: 'var(--font-mono)' }}>{item.subtitle}</span>
                        <span>•</span>
                        <span style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
                          <Clock size={10} /> {item.timestamp}
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Actions */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexShrink: 0 }}>
                    {/* Replay / Open in Studio */}
                    <button
                      onClick={() => onReplayItem(item)}
                      className="btn-secondary"
                      style={{ padding: '4px 8px', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}
                      title="Open and replay in Studio"
                    >
                      <Play size={11} color="#10b981" />
                      <span>Replay</span>
                    </button>

                    {/* Copy */}
                    <button
                      onClick={() => handleCopy(item.id, item.title)}
                      className="sidebar-action-btn"
                      style={{ padding: '6px' }}
                      title="Copy query / command to clipboard"
                    >
                      {copiedId === item.id ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
                    </button>

                    {/* Delete item */}
                    <button
                      onClick={() => onDeleteHistoryItem(item.id)}
                      className="sidebar-action-btn"
                      style={{ padding: '6px' }}
                      title="Remove from history"
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
