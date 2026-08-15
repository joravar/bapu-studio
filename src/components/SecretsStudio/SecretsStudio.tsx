import React, { useState } from 'react';
import { 
  KeyRound, 
  Eye, 
  EyeOff, 
  Plus, 
  Trash2, 
  Copy, 
  Check, 
  Download, 
  ShieldAlert,
  FolderLock
} from 'lucide-react';
import { Environment, KeyValuePair } from '../../types';

interface SecretsStudioProps {
  environments: Environment[];
  activeEnv: Environment;
  onUpdateEnvironment: (updated: Environment) => void;
}

export const SecretsStudio: React.FC<SecretsStudioProps> = ({
  environments,
  activeEnv,
  onUpdateEnvironment
}) => {
  const [showSecrets, setShowSecrets] = useState<Record<string, boolean>>({});
  const [copied, setCopied] = useState(false);

  const toggleShowSecret = (id: string) => {
    setShowSecrets(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const handleAddVar = () => {
    const newVar: KeyValuePair = {
      id: `var-${Date.now()}`,
      key: 'NEW_VARIABLE_KEY',
      value: 'value',
      enabled: true,
      isSecret: false
    };
    onUpdateEnvironment({
      ...activeEnv,
      variables: [...activeEnv.variables, newVar]
    });
  };

  const handleExportDotEnv = () => {
    const content = activeEnv.variables
      .filter(v => v.enabled)
      .map(v => `${v.key}=${v.value}`)
      .join('\n');
    
    const element = document.createElement('a');
    const file = new Blob([content], { type: 'text/plain' });
    element.href = URL.createObjectURL(file);
    element.download = `.env.${activeEnv.name.toLowerCase().replace(/\s+/g, '-')}`;
    document.body.appendChild(element);
    element.click();
    document.body.removeChild(element);
  };

  const handleCopyDotEnv = () => {
    const content = activeEnv.variables
      .filter(v => v.enabled)
      .map(v => `${v.key}=${v.value}`)
      .join('\n');
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div style={{ padding: '24px', maxWidth: '960px', margin: '0 auto', width: '100%', height: '100%', overflowY: 'auto' }}>
      {/* Header Banner */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '20px' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <FolderLock size={20} color="#3b82f6" />
            <h2 style={{ fontSize: '18px', fontWeight: 700, color: '#fff' }}>
              Environment & Secrets Matrix
            </h2>
          </div>
          <p style={{ color: 'var(--text-muted)', fontSize: '12px', marginTop: '4px' }}>
            Active Environment: <strong style={{ color: '#60a5fa' }}>{activeEnv.name}</strong> • Local-first, Git-safe, zero cloud leaks.
          </p>
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <button onClick={handleCopyDotEnv} className="btn-secondary">
            {copied ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
            <span>Copy as .env</span>
          </button>
          <button onClick={handleExportDotEnv} className="btn-secondary">
            <Download size={13} />
            <span>Export File</span>
          </button>
          <button onClick={handleAddVar} className="btn-send" style={{ padding: '6px 14px', fontSize: '12px' }}>
            <Plus size={13} />
            <span>Add Variable</span>
          </button>
        </div>
      </div>

      {/* Variables Table */}
      <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-lg)', overflow: 'hidden' }}>
        <table className="kv-table">
          <thead>
            <tr>
              <th style={{ width: '40px' }}>State</th>
              <th style={{ width: '220px' }}>Variable Key</th>
              <th>Value</th>
              <th style={{ width: '80px', textAlign: 'center' }}>Secret</th>
              <th style={{ width: '50px' }}></th>
            </tr>
          </thead>
          <tbody>
            {activeEnv.variables.map((v, idx) => {
              const isMasked = v.isSecret && !showSecrets[v.id];
              return (
                <tr key={v.id}>
                  <td>
                    <input
                      type="checkbox"
                      checked={v.enabled}
                      onChange={(e) => {
                        const updated = [...activeEnv.variables];
                        updated[idx].enabled = e.target.checked;
                        onUpdateEnvironment({ ...activeEnv, variables: updated });
                      }}
                    />
                  </td>
                  <td>
                    <input
                      type="text"
                      value={v.key}
                      onChange={(e) => {
                        const updated = [...activeEnv.variables];
                        updated[idx].key = e.target.value;
                        onUpdateEnvironment({ ...activeEnv, variables: updated });
                      }}
                      className="kv-input"
                      style={{ fontFamily: 'var(--font-mono)', fontWeight: 600 }}
                    />
                  </td>
                  <td>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                      <input
                        type={isMasked ? 'password' : 'text'}
                        value={v.value}
                        onChange={(e) => {
                          const updated = [...activeEnv.variables];
                          updated[idx].value = e.target.value;
                          onUpdateEnvironment({ ...activeEnv, variables: updated });
                        }}
                        className="kv-input"
                        style={{ fontFamily: 'var(--font-mono)', color: isMasked ? 'var(--text-dim)' : '#38bdf8' }}
                      />
                      {v.isSecret && (
                        <button
                          onClick={() => toggleShowSecret(v.id)}
                          className="sidebar-action-btn"
                          title={isMasked ? 'Show Secret Value' : 'Hide Secret Value'}
                        >
                          {isMasked ? <Eye size={13} /> : <EyeOff size={13} />}
                        </button>
                      )}
                    </div>
                  </td>
                  <td style={{ textAlign: 'center' }}>
                    <input
                      type="checkbox"
                      checked={!!v.isSecret}
                      onChange={(e) => {
                        const updated = [...activeEnv.variables];
                        updated[idx].isSecret = e.target.checked;
                        onUpdateEnvironment({ ...activeEnv, variables: updated });
                      }}
                    />
                  </td>
                  <td>
                    <button
                      onClick={() => {
                        const updated = activeEnv.variables.filter((_, i) => i !== idx);
                        onUpdateEnvironment({ ...activeEnv, variables: updated });
                      }}
                      className="sidebar-action-btn"
                    >
                      <Trash2 size={13} />
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div style={{
        marginTop: '16px',
        padding: '12px 16px',
        background: 'rgba(59, 130, 246, 0.05)',
        border: '1px solid rgba(59, 130, 246, 0.2)',
        borderRadius: 'var(--radius-md)',
        display: 'flex',
        alignItems: 'center',
        gap: '10px',
        fontSize: '12px',
        color: 'var(--text-muted)'
      }}>
        <ShieldAlert size={16} color="#60a5fa" />
        <span>
          Use <code>{"{{VARIABLE_NAME}}"}</code> inside request URLs, headers, or body payloads to automatically interpolate variables during test execution.
        </span>
      </div>
    </div>
  );
};
