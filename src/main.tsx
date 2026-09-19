import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import { applyTheme, getStoredTheme } from './utils/theme';
import './index.css';

// Applied before the first render so the page paints in the right theme immediately,
// instead of flashing dark and then switching to a saved light preference.
applyTheme(getStoredTheme());

const rootElement = document.getElementById('root');
if (rootElement) {
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <ErrorBoundary fallbackTitle="Bapu Studio encountered an unexpected application error">
        <App />
      </ErrorBoundary>
    </React.StrictMode>
  );
}
