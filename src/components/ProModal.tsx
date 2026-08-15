import React, { useState, useEffect } from 'react';
import { 
  X, 
  Check, 
  Sparkles, 
  ShieldCheck, 
  Users, 
  Zap, 
  Cloud, 
  Lock,
  Key,
  ExternalLink,
  CheckCircle2
} from 'lucide-react';
import { LicenseManager, LicenseStatus } from '../utils/licenseManager';

interface ProModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const ProModal: React.FC<ProModalProps> = ({ isOpen, onClose }) => {
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus>(LicenseManager.getStoredLicense());
  const [keyInput, setKeyInput] = useState('');
  const [isActivating, setIsActivating] = useState(false);
  const [feedback, setFeedback] = useState<{ success: boolean; message: string } | null>(null);

  useEffect(() => {
    if (isOpen) {
      setLicenseStatus(LicenseManager.getStoredLicense());
      setFeedback(null);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleActivate = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsActivating(true);
    setFeedback(null);

    const result = await LicenseManager.activateLicense(keyInput);
    setIsActivating(false);
    setFeedback(result);

    if (result.success) {
      setLicenseStatus(LicenseManager.getStoredLicense());
    }
  };

  const handleOpenCheckout = () => {
    // Open Lemon Squeezy / Stripe checkout page
    window.open('https://bapustudio.dev/pricing', '_blank');
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" style={{ maxWidth: '720px' }} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
          <div>
            <div className="pro-badge-glow">
              <Sparkles size={12} />
              <span>Commercial Open-Source Edition</span>
            </div>
            <h2 style={{ fontSize: '22px', fontWeight: 800, color: '#fff', marginTop: '8px' }}>
              Level Up with Bapu Pro & Team Sync
            </h2>
            <p style={{ color: 'var(--text-muted)', fontSize: '13px', marginTop: '4px' }}>
              Keep the core client 100% free and open-source forever. Upgrade for multi-device sync and enterprise security.
            </p>
          </div>

          <button onClick={onClose} className="sidebar-action-btn" style={{ padding: '6px' }}>
            <X size={18} />
          </button>
        </div>

        {/* License Activated Banner */}
        {licenseStatus.isValid ? (
          <div style={{
            margin: '20px 0',
            padding: '16px 20px',
            background: 'rgba(16, 185, 129, 0.1)',
            border: '1px solid rgba(16, 185, 129, 0.3)',
            borderRadius: 'var(--radius-lg)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between'
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <CheckCircle2 size={24} color="#10b981" />
              <div>
                <div style={{ fontWeight: 700, color: '#10b981', fontSize: '14px' }}>
                  Bapu Pro is Active
                </div>
                <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '2px' }}>
                  Key: <code>{licenseStatus.licenseKey?.slice(0, 10)}••••••••</code> • Unlimited device sync enabled
                </div>
              </div>
            </div>

            <button
              onClick={() => {
                LicenseManager.deactivateLicense();
                setLicenseStatus(LicenseManager.getStoredLicense());
              }}
              className="btn-secondary"
              style={{ fontSize: '11px', padding: '4px 10px' }}
            >
              Deactivate
            </button>
          </div>
        ) : (
          <>
            {/* Pricing Comparison Grid */}
            <div className="pro-pricing-grid">
              {/* Pro Tier */}
              <div className="pricing-card featured">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: '14px', fontWeight: 700, color: '#fff' }}>Bapu Pro</span>
                  <span style={{ fontSize: '10px', background: 'rgba(59,130,246,0.2)', color: '#60a5fa', padding: '2px 6px', borderRadius: '4px', fontWeight: 700 }}>INDIVIDUAL</span>
                </div>
                <div className="pricing-price">
                  $12 <span>/ month</span>
                </div>
                <ul className="feature-list">
                  <li><Check size={14} /> End-to-End Encrypted Cloud Sync</li>
                  <li><Check size={14} /> Unlimited Device Vault Pairing</li>
                  <li><Check size={14} /> Hosted Cloud Mock Servers</li>
                  <li><Check size={14} /> AI Query & Regex Generator</li>
                  <li><Check size={14} /> Priority Support & Discord Badge</li>
                </ul>
                <button onClick={handleOpenCheckout} className="btn-upgrade" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px' }}>
                  <span>Upgrade to Pro ($12/mo)</span>
                  <ExternalLink size={13} />
                </button>
              </div>

              {/* Team / Enterprise Tier */}
              <div className="pricing-card">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: '14px', fontWeight: 700, color: '#fff' }}>Team & Enterprise</span>
                  <span style={{ fontSize: '10px', background: 'rgba(16,185,129,0.2)', color: '#10b981', padding: '2px 6px', borderRadius: '4px', fontWeight: 700 }}>TEAMS</span>
                </div>
                <div className="pricing-price">
                  $25 <span>/ seat / mo</span>
                </div>
                <ul className="feature-list">
                  <li><Check size={14} /> Everything in Pro tier</li>
                  <li><Check size={14} /> Role-Based Access Control (RBAC)</li>
                  <li><Check size={14} /> Production DB Credential Guard</li>
                  <li><Check size={14} /> SSO / SAML & Okta Integration</li>
                  <li><Check size={14} /> SOC2 Compliant Audit Logs</li>
                </ul>
                <button onClick={handleOpenCheckout} className="btn-secondary" style={{ width: '100%', justifyContent: 'center', padding: '10px', fontWeight: 700 }}>
                  Contact Enterprise Sales
                </button>
              </div>
            </div>

            {/* License Key Activation Box */}
            <div style={{
              background: 'var(--bg-input)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-lg)',
              padding: '16px 20px',
              margin: '16px 0 20px'
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', fontWeight: 600, color: '#fff', marginBottom: '8px' }}>
                <Key size={14} color="#f59e0b" />
                <span>Already have a License Key?</span>
              </div>

              <form onSubmit={handleActivate} style={{ display: 'flex', gap: '8px' }}>
                <input
                  type="text"
                  value={keyInput}
                  onChange={(e) => setKeyInput(e.target.value)}
                  placeholder="Paste your license key (e.g. BAPU-PRO-XXXX-XXXX)"
                  className="url-input"
                  style={{
                    background: 'var(--bg-card)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 'var(--radius-md)',
                    padding: '8px 12px',
                    color: '#fff',
                    flex: 1
                  }}
                />
                <button type="submit" disabled={isActivating || !keyInput.trim()} className="btn-send" style={{ padding: '8px 18px' }}>
                  {isActivating ? 'Verifying...' : 'Activate Key'}
                </button>
              </form>

              {feedback && (
                <div style={{
                  marginTop: '10px',
                  fontSize: '12px',
                  color: feedback.success ? '#10b981' : '#ef4444'
                }}>
                  {feedback.message}
                </div>
              )}
            </div>
          </>
        )}

        {/* Footer Guarantee */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          color: 'var(--text-dim)',
          fontSize: '11px',
          borderTop: '1px solid var(--border-subtle)',
          paddingTop: '16px'
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <Lock size={12} color="#10b981" />
            <span>Zero Telemetry on Free Core</span>
          </div>
          <span>•</span>
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <Cloud size={12} color="#60a5fa" />
            <span>Self-Hostable Sync Server Available</span>
          </div>
        </div>
      </div>
    </div>
  );
};
