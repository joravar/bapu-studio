import React, { useState, useEffect, useRef } from 'react';
import {
  Radio,
  Send,
  Play,
  Square,
  Clock,
  ArrowUpRight,
  ArrowDownLeft,
  Trash2
} from 'lucide-react';

interface StreamMessage {
  id: string;
  type: 'sent' | 'received' | 'system';
  content: string;
  timestamp: string;
  latencyMs?: number;
}

export const StreamStudio: React.FC = () => {
  const [streamType, setStreamType] = useState<'sse' | 'ws'>('sse');
  const [url, setUrl] = useState('https://api.example.com/v1/chat/completions/stream');
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [outMessage, setOutMessage] = useState('{"type": "subscribe", "channel": "live_feed"}');
  const [messages, setMessages] = useState<StreamMessage[]>([
    {
      id: 'm-1',
      type: 'system',
      content: 'Ready to connect to real-time streaming endpoint.',
      timestamp: '00:00:00'
    }
  ]);

  const esRef = useRef<EventSource | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const connectStartRef = useRef<number>(0);

  const addMessage = (type: StreamMessage['type'], content: string, latencyMs?: number) => {
    setMessages(prev => [
      ...prev,
      {
        id: `m-${Date.now()}-${Math.random()}`,
        type,
        content,
        timestamp: new Date().toLocaleTimeString(),
        latencyMs
      }
    ]);
  };

  const closeActiveConnection = () => {
    if (esRef.current) {
      esRef.current.close();
      esRef.current = null;
    }
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }
    setIsConnected(false);
    setIsConnecting(false);
  };

  const handleToggleConnect = () => {
    if (isConnected || isConnecting) {
      closeActiveConnection();
      addMessage('system', 'Disconnected from stream.');
      return;
    }

    const trimmedUrl = url.trim();
    if (!trimmedUrl) {
      addMessage('system', 'Enter a stream URL before connecting.');
      return;
    }

    if (streamType === 'sse') {
      if (!/^https?:\/\//i.test(trimmedUrl)) {
        addMessage('system', `"${trimmedUrl}" is not a valid SSE URL — it must start with http:// or https://.`);
        return;
      }

      setIsConnecting(true);
      connectStartRef.current = performance.now();
      let es: EventSource;
      try {
        es = new EventSource(trimmedUrl);
      } catch (err: any) {
        setIsConnecting(false);
        addMessage('system', `Failed to open SSE connection: ${err?.message || 'invalid URL'}`);
        return;
      }
      esRef.current = es;

      es.onopen = () => {
        setIsConnecting(false);
        setIsConnected(true);
        addMessage('system', `Connected to ${trimmedUrl} (SSE)`);
      };
      es.onmessage = (evt) => {
        addMessage('received', evt.data, Math.round(performance.now() - connectStartRef.current));
      };
      es.onerror = () => {
        addMessage('system', `Stream error or connection closed for ${trimmedUrl}. Verify the URL is reachable and sends CORS headers permitting this app.`);
        es.close();
        esRef.current = null;
        setIsConnected(false);
        setIsConnecting(false);
      };
    } else {
      if (!/^wss?:\/\//i.test(trimmedUrl)) {
        addMessage('system', `"${trimmedUrl}" is not a valid WebSocket URL — it must start with ws:// or wss://.`);
        return;
      }

      setIsConnecting(true);
      let ws: WebSocket;
      try {
        ws = new WebSocket(trimmedUrl);
      } catch (err: any) {
        setIsConnecting(false);
        addMessage('system', `Failed to open WebSocket: ${err?.message || 'invalid URL'}`);
        return;
      }
      wsRef.current = ws;

      ws.onopen = () => {
        setIsConnecting(false);
        setIsConnected(true);
        addMessage('system', `Connected to ${trimmedUrl} (WebSocket)`);
      };
      ws.onmessage = (evt) => {
        addMessage('received', typeof evt.data === 'string' ? evt.data : '[binary frame]');
      };
      ws.onerror = () => {
        addMessage('system', `WebSocket error for ${trimmedUrl}. Verify the URL is reachable.`);
      };
      ws.onclose = (evt) => {
        addMessage('system', `Disconnected from stream${evt.code ? ` (code ${evt.code}${evt.reason ? `: ${evt.reason}` : ''})` : ''}.`);
        wsRef.current = null;
        setIsConnected(false);
        setIsConnecting(false);
      };
    }
  };

  const handleSendMessage = () => {
    if (!outMessage.trim()) return;
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      addMessage('system', 'Cannot send — WebSocket is not connected.');
      return;
    }
    wsRef.current.send(outMessage);
    addMessage('sent', outMessage);
  };

  const handleClear = () => {
    setMessages([]);
  };

  // Close any live connection when switching SSE/WS mode, so it doesn't keep running in the background.
  const handleStreamTypeChange = (nextType: 'sse' | 'ws') => {
    closeActiveConnection();
    setStreamType(nextType);
    setUrl(nextType === 'sse'
      ? 'https://api.example.com/v1/chat/completions/stream'
      : 'wss://echo.websocket.events');
  };

  useEffect(() => {
    return () => {
      esRef.current?.close();
      wsRef.current?.close();
    };
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Top Connection Bar */}
      <div className="request-bar-container">
        <select
          value={streamType}
          onChange={(e) => handleStreamTypeChange(e.target.value as 'sse' | 'ws')}
          className="method-select-dropdown"
        >
          <option value="sse">SSE Stream</option>
          <option value="ws">WebSocket</option>
        </select>

        <div className="url-input-wrapper">
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="ws:// or https:// stream URL"
            className="url-input"
            disabled={isConnected || isConnecting}
          />
        </div>

        <button
          onClick={handleToggleConnect}
          disabled={isConnecting}
          className={isConnected ? "btn-secondary" : "btn-send"}
          style={{
            background: isConnected ? 'rgba(239, 68, 68, 0.2)' : undefined,
            color: isConnected ? '#ef4444' : undefined,
            borderColor: isConnected ? 'rgba(239, 68, 68, 0.4)' : undefined
          }}
        >
          {isConnected ? <Square size={13} /> : <Play size={13} />}
          <span>{isConnecting ? 'Connecting...' : isConnected ? 'Disconnect' : 'Connect'}</span>
        </button>
      </div>

      {/* Stream Split View */}
      <div className="split-workspace-pane">
        {/* Left: WebSocket Frame Composer */}
        {streamType === 'ws' && (
          <div className="request-config-panel" style={{ maxWidth: '380px' }}>
            <div className="panel-tab-header">
              <span style={{ fontSize: '12px', fontWeight: 600 }}>Message Payload</span>
            </div>
            <div style={{ flex: 1, padding: '12px', display: 'flex', flexDirection: 'column' }}>
              <textarea
                value={outMessage}
                onChange={(e) => setOutMessage(e.target.value)}
                placeholder="Message to send..."
                className="code-textarea"
                style={{ flex: 1, background: 'var(--bg-input)', padding: '8px', borderRadius: 'var(--radius-sm)' }}
              />
              <button
                onClick={handleSendMessage}
                disabled={!isConnected}
                className="btn-send"
                style={{ marginTop: '10px', justifyContent: 'center' }}
              >
                <Send size={13} />
                <span>Send Frame</span>
              </button>
            </div>
          </div>
        )}

        {/* Right: Live Event Timeline */}
        <div className="response-viewer-panel" style={{ flex: 1 }}>
          <div className="panel-tab-header">
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Radio size={14} color={isConnected ? '#10b981' : 'var(--text-dim)'} />
              <span style={{ fontSize: '12px', fontWeight: 600 }}>
                Live Stream Event Timeline ({messages.length})
              </span>
            </div>

            <button onClick={handleClear} className="sidebar-action-btn" title="Clear Stream Logs">
              <Trash2 size={13} />
            </button>
          </div>

          <div className="editor-container" style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {messages.map((msg) => (
              <div
                key={msg.id}
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: '8px',
                  padding: '6px 10px',
                  borderRadius: 'var(--radius-sm)',
                  background: msg.type === 'sent'
                    ? 'rgba(59, 130, 246, 0.08)'
                    : msg.type === 'received'
                    ? 'rgba(16, 185, 129, 0.08)'
                    : 'rgba(255, 255, 255, 0.03)',
                  border: '1px solid rgba(255, 255, 255, 0.04)',
                  fontSize: '12px',
                  fontFamily: 'var(--font-mono)'
                }}
              >
                <span style={{ marginTop: '2px' }}>
                  {msg.type === 'sent' && <ArrowUpRight size={13} color="#3b82f6" />}
                  {msg.type === 'received' && <ArrowDownLeft size={13} color="#10b981" />}
                  {msg.type === 'system' && <Clock size={13} color="var(--text-dim)" />}
                </span>

                <div style={{ flex: 1, wordBreak: 'break-all' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-dim)', fontSize: '10px', marginBottom: '2px' }}>
                    <span>{msg.type.toUpperCase()}</span>
                    <span>{msg.timestamp} {msg.latencyMs ? `• ${msg.latencyMs}ms` : ''}</span>
                  </div>
                  <span style={{ color: msg.type === 'received' ? '#38bdf8' : msg.type === 'sent' ? '#f1f5f9' : 'var(--text-muted)' }}>
                    {msg.content}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
