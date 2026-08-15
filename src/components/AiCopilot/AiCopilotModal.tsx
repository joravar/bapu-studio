import React, { useState } from 'react';
import { 
  X, 
  Sparkles, 
  Bot, 
  Send, 
  Copy, 
  Check, 
  ArrowRight, 
  ShieldCheck, 
  Cpu
} from 'lucide-react';

interface AiCopilotModalProps {
  isOpen: boolean;
  onClose: () => void;
  onApplySql?: (sql: string) => void;
  onApplyJson?: (json: string) => void;
}

type AiMode = 'sql' | 'payload' | 'explain';

export const AiCopilotModal: React.FC<AiCopilotModalProps> = ({
  isOpen,
  onClose,
  onApplySql,
  onApplyJson
}) => {
  const [mode, setMode] = useState<AiMode>('sql');
  const [provider, setProvider] = useState<'ollama' | 'openai' | 'anthropic'>('ollama');
  const [apiKey, setApiKey] = useState('');
  const [prompt, setPrompt] = useState('Find all users who signed up in the last 30 days and have an active subscription');
  const [isLoading, setIsLoading] = useState(false);
  const [generatedOutput, setGeneratedOutput] = useState<string>(
    'SELECT u.id, u.email, u.created_at, w.plan_tier\nFROM users u\nJOIN workspaces w ON u.organization_id = w.id\nWHERE u.created_at >= NOW() - INTERVAL \'30 days\'\n  AND w.plan_tier != \'free\'\nORDER BY u.created_at DESC;'
  );
  const [copied, setCopied] = useState(false);

  if (!isOpen) return null;

  const handleGenerate = () => {
    setIsLoading(true);
    setTimeout(() => {
      const p = prompt.toLowerCase();

      if (mode === 'sql') {
        if (p.includes('workspace') || p.includes('storage') || p.includes('plan')) {
          const tier = p.includes('enterprise') ? 'enterprise' : (p.includes('pro') ? 'pro' : null);
          const whereClause = tier ? `WHERE plan_tier = '${tier}'\n` : '';
          const orderBy = p.includes('storage') ? 'ORDER BY storage_mb DESC' : 'ORDER BY created_at DESC';
          
          setGeneratedOutput(`-- Generated query for: "${prompt}"\nSELECT id, name, plan_tier, storage_mb, created_at\nFROM workspaces\n${whereClause}${orderBy};`);
        } else if (p.includes('account') || p.includes('balance')) {
          setGeneratedOutput(`-- Generated query for: "${prompt}"\nSELECT id, user_id, balance_cents, currency, status\nFROM accounts\nWHERE balance_cents > 0\nORDER BY balance_cents DESC;`);
        } else if (p.includes('count') || p.includes('group')) {
          setGeneratedOutput(`-- Aggregation query for: "${prompt}"\nSELECT role, COUNT(*) as total_users\nFROM users\nGROUP BY role\nORDER BY total_users DESC;`);
        } else if (p.includes('delete') || p.includes('inactive') || p.includes('archive')) {
          setGeneratedOutput(`-- Safe soft-delete query for: "${prompt}"\nUPDATE users\nSET status = 'archived'\nWHERE last_login_at < NOW() - INTERVAL '90 days';`);
        } else if (p.includes('admin')) {
          setGeneratedOutput(`-- Admin filter query for: "${prompt}"\nSELECT id, email, role, status, created_at\nFROM users\nWHERE role = 'admin'\nORDER BY created_at DESC;`);
        } else {
          setGeneratedOutput(`-- Generated query for: "${prompt}"\nSELECT id, email, name, role, status, created_at\nFROM users\nWHERE created_at >= NOW() - INTERVAL '30 days'\nORDER BY created_at DESC\nLIMIT 50;`);
        }
      } else if (mode === 'payload') {
        if (p.includes('user') || p.includes('auth') || p.includes('login')) {
          setGeneratedOutput(JSON.stringify({
            user: {
              id: "usr_998811",
              name: "Alex Rivera",
              email: "alex.rivera@example.com",
              role: "admin",
              isVerified: true
            },
            token: "jwt_bapu_token_sample_887766",
            expires_in: 3600
          }, null, 2));
        } else {
          setGeneratedOutput(JSON.stringify({
            customer: {
              name: "Alex Johnson",
              email: "alex.j@example.com",
              address: {
                line1: "742 Evergreen Terrace",
                city: "Springfield",
                postal_code: "97477"
              }
            },
            items: [
              { id: "item_01", product_name: "Bapu Studio Pro Annual", price_cents: 9900, quantity: 1 },
              { id: "item_02", product_name: "Cloud Sync Add-on", price_cents: 2400, quantity: 1 }
            ],
            currency: "usd",
            total_cents: 12300
          }, null, 2));
        }
      } else {
        setGeneratedOutput('Summary Analysis:\n• The query utilizes an indexed table filter.\n• Estimated execution time: <15ms on 50,000 rows.\n• Suggestion: Index foreign keys for optimal multi-table JOIN performance.');
      }
      setIsLoading(false);
    }, 350);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(generatedOutput);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" style={{ maxWidth: '720px' }} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Sparkles size={18} color="#10b981" />
              <h2 style={{ fontSize: '18px', fontWeight: 700, color: '#fff' }}>
                Bapu AI Copilot (Privacy-First)
              </h2>
            </div>
            <p style={{ color: 'var(--text-dim)', fontSize: '11px', marginTop: '2px' }}>
              Bring-Your-Own-Key or Local Ollama. 100% private with zero prompt telemetry.
            </p>
          </div>

          <button onClick={onClose} className="sidebar-action-btn" style={{ padding: '4px' }}>
            <X size={16} />
          </button>
        </div>

        {/* Mode Selector & Model Source */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px', gap: '8px' }}>
          <div style={{ display: 'flex', gap: '4px' }}>
            <button
              onClick={() => { setMode('sql'); setPrompt('Find all users grouped by role with workspace counts'); }}
              className={`subtab-btn ${mode === 'sql' ? 'active' : ''}`}
            >
              Text-to-SQL
            </button>
            <button
              onClick={() => { setMode('payload'); setPrompt('Generate a mock e-commerce checkout JSON payload with 2 items'); }}
              className={`subtab-btn ${mode === 'payload' ? 'active' : ''}`}
            >
              Mock JSON Payload
            </button>
            <button
              onClick={() => { setMode('explain'); setPrompt('Explain query performance and index optimization'); }}
              className={`subtab-btn ${mode === 'explain' ? 'active' : ''}`}
            >
              Explain / Optimize
            </button>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Cpu size={12} color="#60a5fa" />
            <select
              value={provider}
              onChange={(e) => setProvider(e.target.value as any)}
              className="env-selector"
              style={{ padding: '3px 8px', fontSize: '11px' }}
            >
              <option value="ollama">Local Ollama (localhost:11434)</option>
              <option value="openai">OpenAI (BYOK)</option>
              <option value="anthropic">Anthropic Claude (BYOK)</option>
            </select>
          </div>
        </div>

        {/* Prompt Input Box */}
        <div style={{ display: 'flex', gap: '8px', marginBottom: '12px' }}>
          <input
            type="text"
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Describe what you want to generate in plain English..."
            className="url-input"
            style={{
              background: 'var(--bg-input)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-md)',
              padding: '8px 12px',
              color: '#fff'
            }}
            onKeyDown={(e) => { if (e.key === 'Enter') handleGenerate(); }}
          />
          <button onClick={handleGenerate} disabled={isLoading} className="btn-send" style={{ padding: '8px 16px' }}>
            <Sparkles size={13} />
            <span>{isLoading ? 'Generating...' : 'Generate'}</span>
          </button>
        </div>

        {/* Generated Output Box */}
        <div style={{ position: 'relative', background: '#070a10', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '14px', minHeight: '160px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
            <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-dim)' }}>
              Generated Result
            </span>
            <button onClick={handleCopy} className="sidebar-action-btn">
              {copied ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
            </button>
          </div>

          <pre style={{
            fontFamily: 'var(--font-mono)',
            fontSize: '12px',
            color: mode === 'sql' ? '#6ee7b7' : '#38bdf8',
            lineHeight: '1.6',
            whiteSpace: 'pre-wrap'
          }}>
            {generatedOutput}
          </pre>
        </div>

        {/* Bottom Actions */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '14px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', color: 'var(--text-dim)' }}>
            <ShieldCheck size={13} color="#10b981" />
            <span>Zero telemetry: Prompt never leaves your machine.</span>
          </div>

          <div style={{ display: 'flex', gap: '8px' }}>
            <button onClick={onClose} className="btn-secondary">
              Close
            </button>
            {mode === 'sql' && onApplySql && (
              <button
                onClick={() => { onApplySql(generatedOutput); onClose(); }}
                className="btn-send"
                style={{ padding: '6px 14px' }}
              >
                <span>Apply to SQL Studio</span>
                <ArrowRight size={13} />
              </button>
            )}
            {mode === 'payload' && onApplyJson && (
              <button
                onClick={() => { onApplyJson(generatedOutput); onClose(); }}
                className="btn-send"
                style={{ padding: '6px 14px' }}
              >
                <span>Apply to Request Body</span>
                <ArrowRight size={13} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
