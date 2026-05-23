import React, { Component, type ErrorInfo, type ReactNode } from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { I18nProvider } from './i18n/context';
import { readStoredLocale, translateKey } from './i18n/translations';
import './index.css';
import './styles/phone-dock.css';

function crashT(key: string): string {
  return translateKey(readStoredLocale(), key);
}

/** Évite l’écran blanc si une erreur React remonte depuis l’app. */
class RootErrorBoundary extends Component<{ children: ReactNode }, { err: Error | null }> {
  state: { err: Error | null } = { err: null };

  static getDerivedStateFromError(err: Error) {
    return { err };
  }

  componentDidCatch(err: Error, info: ErrorInfo) {
    console.error('[PanicBase]', err, info.componentStack);
  }

  render() {
    if (this.state.err) {
      return (
        <div className="flex min-h-screen flex-col gap-3 bg-base-100 p-6 text-base-content" data-theme="panicbase">
          <h1 className="font-sora text-lg font-bold">{crashT('error.displayTitle')}</h1>
          <pre className="max-h-[50vh] overflow-auto rounded-lg bg-base-200/80 p-3 font-mono text-xs">
            {this.state.err.message}
          </pre>
          <button
            type="button"
            className="btn btn-primary btn-sm w-fit"
            onClick={() => this.setState({ err: null })}
          >
            {crashT('error.retry')}
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <RootErrorBoundary>
        <App />
      </RootErrorBoundary>
    </I18nProvider>
  </React.StrictMode>,
);
