import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '../i18n/context';

type Props = {
  open: boolean;
  onClose: () => void;
  logIndex: number | null;
  fileName: string | null;
  /** Optional in-memory text (skips device read when set) */
  inlineText?: string | null;
};

export function PanicLogSourceModal({ open, onClose, logIndex, fileName, inlineText }: Props) {
  const { t } = useI18n();
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const taRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!open) {
      setText('');
      setErr('');
      return;
    }
    const inline = inlineText?.trim();
    if (inline) {
      setBusy(false);
      setErr('');
      setText(inlineText ?? '');
      return;
    }
    if (logIndex === null) {
      setText('');
      setErr('');
      setBusy(false);
      return;
    }
    setBusy(true);
    setErr('');
    setText('');
    invoke<string>('read_pulled_device_panic_raw', { index: logIndex })
      .then(setText)
      .catch((e) => setErr(String(e)))
      .finally(() => setBusy(false));
  }, [open, logIndex, inlineText]);

  const copyStr = async (chunk: string) => {
    if (!chunk) return;
    try {
      await navigator.clipboard.writeText(chunk);
    } catch {
      /* ignore */
    }
  };

  const copySelection = () => {
    const el = taRef.current;
    if (!el) return;
    const a = el.selectionStart ?? 0;
    const b = el.selectionEnd ?? 0;
    if (a === b) return;
    void copyStr(el.value.slice(Math.min(a, b), Math.max(a, b)));
  };

  const copyCurrentLine = () => {
    const el = taRef.current;
    if (!el || !el.value) return;
    const pos = Math.max(0, (el.selectionEnd ?? el.selectionStart ?? 0) - 1);
    const v = el.value;
    const start = v.lastIndexOf('\n', pos - 1) + 1;
    const nl = v.indexOf('\n', pos);
    const end = nl < 0 ? v.length : nl;
    void copyStr(v.slice(start, end));
  };

  if (!open) return null;

  return (
    <div className="modal modal-open z-[300]" role="dialog" aria-modal="true" aria-labelledby="log-viewer-title">
      <div className="modal-box flex max-h-[min(92vh,880px)] w-[min(96vw,920px)] max-w-none flex-col gap-3 p-4 sm:p-6">
        <header className="shrink-0">
          <h2 id="log-viewer-title" className="font-outfit text-lg font-bold tracking-tight">
            {t('logViewer.title')}
          </h2>
          {fileName ? (
            <p className="mt-1 mb-0 truncate font-mono text-[11px] font-medium text-base-content/75">{fileName}</p>
          ) : null}
          <p className="mt-2 mb-0 text-xs leading-snug text-base-content/55">{t('logViewer.hint')}</p>
        </header>

        <div className="min-h-0 flex-1 overflow-hidden">
          {busy ? (
            <p className="m-0 py-8 text-center text-sm text-base-content/60">{t('logViewer.loading')}</p>
          ) : err ? (
            <p className="m-0 rounded-lg border border-error/30 bg-error/10 p-4 text-sm text-error">
              {t('logViewer.error')} {err}
            </p>
          ) : (
            <textarea
              ref={taRef}
              readOnly
              spellCheck={false}
              className="textarea textarea-bordered h-[min(58vh,520px)] w-full resize-y font-mono text-[11px] leading-relaxed"
              value={text}
              aria-label={t('logViewer.title')}
            />
          )}
        </div>

        <div className="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-base-300 pt-3">
          <button type="button" className="btn btn-ghost btn-sm font-sora" onClick={() => void copyStr(text)} disabled={busy || !!err || !text}>
            {t('logViewer.copyAll')}
          </button>
          <button type="button" className="btn btn-ghost btn-sm font-sora" onClick={copySelection} disabled={busy || !!err || !text}>
            {t('logViewer.copySelection')}
          </button>
          <button type="button" className="btn btn-ghost btn-sm font-sora" onClick={copyCurrentLine} disabled={busy || !!err || !text}>
            {t('logViewer.copyLine')}
          </button>
          <button type="button" className="btn btn-primary btn-sm font-sora" onClick={onClose}>
            {t('logViewer.close')}
          </button>
        </div>
      </div>
      <button type="button" className="modal-backdrop !bg-transparent" aria-label={t('logViewer.close')} onClick={onClose} />
    </div>
  );
}
