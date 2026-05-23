import { useCallback, useEffect, useState } from 'react';
import { useI18n } from '../i18n/context';

export type DeviceInfoField = { id: string; value: string };

export type IphoneDeviceDetailsPayload = {
  fields: DeviceInfoField[];
  udid: string | null;
  hint: string | null;
};

type Props = {
  open: boolean;
  onClose: () => void;
  loading: boolean;
  error: string;
  data: IphoneDeviceDetailsPayload | null;
  onRetry: () => void;
};

function fieldsToMap(fields: DeviceInfoField[]): Record<string, string> {
  const m: Record<string, string> = {};
  for (const f of fields) {
    m[f.id] = f.value;
  }
  return m;
}

function pick(m: Record<string, string>, ...keys: string[]): string | undefined {
  for (const k of keys) {
    const v = m[k];
    if (v !== undefined && v.trim() !== '') return v;
  }
  return undefined;
}

function IdRow({
  label,
  value,
  copyLabel,
  onCopy,
}: {
  label: string;
  value: string | undefined;
  copyLabel: string;
  onCopy: (s: string) => void;
}) {
  const has = Boolean(value?.trim());
  return (
    <div
      className={`flex flex-col gap-3 border-b border-base-content/10 px-5 py-4 transition-colors last:border-b-0 sm:flex-row sm:items-center sm:justify-between ${
        has ? 'bg-base-100/80 hover:bg-base-200/60' : 'bg-base-300/40'
      }`}
    >
      <div className="min-w-0 flex-1">
        <p className="m-0 font-sora text-[11px] font-bold uppercase tracking-[0.14em] text-base-content/45">{label}</p>
        <p className={`mt-1.5 m-0 break-all font-mono text-[14px] leading-snug ${has ? 'text-base-content' : 'text-base-content/35'}`}>
          {has ? value : '—'}
        </p>
      </div>
      <button
        type="button"
        className="btn btn-primary btn-sm h-9 min-h-9 shrink-0 px-5 font-sora font-semibold shadow-md shadow-primary/10"
        disabled={!has}
        onClick={() => has && value && onCopy(value)}
      >
        {copyLabel}
      </button>
    </div>
  );
}

export function ConnectedDeviceInfoModal({ open, onClose, loading, error, data, onRetry }: Props) {
  const { t } = useI18n();
  const [copyToast, setCopyToast] = useState<string | null>(null);

  useEffect(() => {
    if (!open) setCopyToast(null);
  }, [open]);

  const showToast = useCallback((msg: string) => {
    setCopyToast(msg);
    window.setTimeout(() => setCopyToast(null), 2000);
  }, []);

  const copyText = useCallback(
    (s: string) => {
      void navigator.clipboard.writeText(s).then(() => showToast(t('deviceInfo.copiedToast')));
    },
    [showToast, t],
  );

  const map = data?.fields?.length ? fieldsToMap(data.fields) : {};
  const imei1 = pick(map, 'InternationalMobileEquipmentIdentity', 'IMEI', 'MobileEquipmentIdentifier');
  const imei2 = pick(map, 'InternationalMobileEquipmentIdentity2');
  const serial = pick(map, 'SerialNumber');
  const ecid = pick(map, 'ECID');
  const udid = data?.udid?.trim() || pick(map, 'UniqueDeviceID');

  if (!open) return null;

  return (
    <>
      {copyToast ? (
        <div className="toast toast-top toast-end z-[120] whitespace-nowrap" role="status">
          <div className="alert alert-success font-sora text-sm shadow-lg">
            <span>{copyToast}</span>
          </div>
        </div>
      ) : null}

      <div className="modal modal-open z-[300]" role="dialog" aria-modal="true" aria-labelledby="device-info-title">
        <div className="modal-box flex max-h-[min(92vh,720px)] w-full max-w-lg flex-col gap-0 overflow-hidden rounded-2xl border border-primary/20 bg-base-300 p-0 shadow-2xl">
          <header className="relative shrink-0 overflow-hidden border-b border-base-content/10 bg-gradient-to-br from-primary/20 via-base-200 to-base-300 px-6 pb-5 pt-6">
            <div className="pointer-events-none absolute -right-8 -top-12 h-40 w-40 rounded-full bg-secondary/20 blur-3xl" aria-hidden />
            <div className="relative flex items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="m-0 font-sora text-[10px] font-bold uppercase tracking-[0.2em] text-primary">{t('deviceInfo.idsBadge')}</p>
                <h2 id="device-info-title" className="mt-1.5 m-0 font-outfit text-2xl font-bold tracking-tight text-base-content">
                  {t('deviceInfo.mod.identity')}
                </h2>
                <p className="mt-1 m-0 max-w-[42ch] font-sora text-xs leading-relaxed text-base-content/60">{t('deviceInfo.idsLead')}</p>
              </div>
              <button type="button" className="btn btn-circle btn-ghost btn-sm shrink-0 border border-base-content/10" onClick={onClose} aria-label={t('deviceInfo.close')}>
                <svg width="18" height="18" viewBox="0 0 24 24" className="opacity-70" aria-hidden>
                  <path fill="currentColor" d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
                </svg>
              </button>
            </div>
          </header>

          <div className="relative min-h-0 flex-1 overflow-y-auto overscroll-contain bg-base-300">
            {loading ? (
              <div className="flex flex-col items-center justify-center gap-4 px-6 py-20">
                <span className="loading loading-spinner loading-lg text-primary" />
                <p className="m-0 text-center font-sora text-sm text-base-content/55">{t('deviceInfo.loading')}</p>
              </div>
            ) : error ? (
              <div className="space-y-4 p-6">
                <div role="alert" className="alert alert-error text-sm shadow-md">
                  <span>
                    {t('deviceInfo.error')} {error}
                  </span>
                </div>
                <button type="button" className="btn btn-primary btn-block font-sora font-semibold" onClick={() => void onRetry()}>
                  {t('deviceInfo.retry')}
                </button>
              </div>
            ) : data ? (
              <div className="flex flex-col">
                {data.hint ? (
                  <div className="border-b border-warning/25 bg-warning/10 px-5 py-3">
                    <p className="m-0 font-sora text-xs leading-relaxed text-warning">{data.hint}</p>
                  </div>
                ) : null}
                <div className="divide-y divide-base-content/10 rounded-b-2xl border-t border-base-content/5 bg-base-100/50">
                  <IdRow label={t('deviceInfo.dash.row.imei1')} value={imei1} copyLabel={t('deviceInfo.mod.tapCopy')} onCopy={copyText} />
                  <IdRow label={t('deviceInfo.dash.row.imei2')} value={imei2} copyLabel={t('deviceInfo.mod.tapCopy')} onCopy={copyText} />
                  <IdRow label={t('deviceInfo.mod.sn')} value={serial} copyLabel={t('deviceInfo.mod.tapCopy')} onCopy={copyText} />
                  <IdRow label={t('deviceInfo.ecidLabel')} value={ecid} copyLabel={t('deviceInfo.mod.tapCopy')} onCopy={copyText} />
                  <IdRow label={t('deviceInfo.udidLabel')} value={udid} copyLabel={t('deviceInfo.mod.tapCopy')} onCopy={copyText} />
                </div>
              </div>
            ) : null}
          </div>

          <footer className="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-base-content/10 bg-base-200/90 px-4 py-4">
            {!loading && !error && data ? (
              <button type="button" className="btn btn-ghost btn-sm font-sora" onClick={() => void onRetry()}>
                {t('deviceInfo.retry')}
              </button>
            ) : null}
            <button type="button" className="btn btn-primary px-6 font-sora font-semibold" onClick={onClose}>
              {t('deviceInfo.close')}
            </button>
          </footer>
        </div>
        <button type="button" className="modal-backdrop !bg-transparent !backdrop-blur-0" aria-label={t('deviceInfo.close')} onClick={onClose} />
      </div>
    </>
  );
}
