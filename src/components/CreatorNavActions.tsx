import { useCallback, useState } from 'react';
import { open } from '@tauri-apps/plugin-shell';
import { useI18n } from '../i18n/context';

const TIKTOK_URL = 'https://www.tiktok.com/@lalchimiste31';
const TRON_ADDRESS = 'TA89TXYkuwEae7vy37JCUwsqiEx5738zXv';
const CONTACT_EMAIL = 'serviceclient@infophone.store';

async function openExternal(url: string): Promise<void> {
  try {
    await open(url);
  } catch {
    try {
      window.open(url, '_blank', 'noopener,noreferrer');
    } catch {
      /* ignore */
    }
  }
}

async function writeClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.setAttribute('readonly', '');
      ta.style.position = 'fixed';
      ta.style.left = '-9999px';
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand('copy');
      document.body.removeChild(ta);
      return ok;
    } catch {
      return false;
    }
  }
}

function TikTokIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden
    >
      <defs>
        <linearGradient id="pbTikTokGrad" x1="2" y1="4" x2="22" y2="20" gradientUnits="userSpaceOnUse">
          <stop stopColor="#2EE6D6" />
          <stop offset="0.45" stopColor="#2EE6D6" />
          <stop offset="0.55" stopColor="#FF2E63" />
          <stop offset="1" stopColor="#FF2E63" />
        </linearGradient>
      </defs>
      <path
        fill="url(#pbTikTokGrad)"
        d="M19.59 6.69a4.83 4.83 0 0 1-3.77-4.25V2h-3.45v13.67a2.89 2.89 0 0 1-5.2 1.74 2.89 2.89 0 0 1 2.31-4.64 2.93 2.93 0 0 1 .88.13V9.4a6.84 6.84 0 0 0-1-.05A6.33 6.33 0 0 0 5 20.1a6.34 6.34 0 0 0 10.86-4.43v-7a8.16 8.16 0 0 0 4.77 1.52v-3.4a4.85 4.85 0 0 1-1-.1z"
      />
    </svg>
  );
}

function TronIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
      <defs>
        <linearGradient id="pbTronGrad" x1="6" y1="4" x2="26" y2="28" gradientUnits="userSpaceOnUse">
          <stop stopColor="#ff1744" />
          <stop offset="1" stopColor="#ff6d00" />
        </linearGradient>
      </defs>
      <path
        fill="url(#pbTronGrad)"
        d="M16 3.2 28.2 10.2v11.6L16 28.8 3.8 21.8V10.2L16 3.2Zm0 3.4L7.4 11.4v9.2L16 25.4l8.6-4.8v-9.2L16 6.6Z"
      />
    </svg>
  );
}

function MailIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      aria-hidden
    >
      <rect x="3.5" y="5.5" width="17" height="13" rx="1.5" strokeLinejoin="round" />
      <path d="M4 7.5 12 13l8-5.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function CheckMini({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" aria-hidden>
      <path d="M20 6L9 17l-5-5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

const navBtnClass =
  'btn btn-ghost btn-sm h-8 min-h-8 gap-1.5 rounded-lg border border-base-content/15 bg-base-100/85 px-2 font-normal text-base-content shadow-[0_1px_2px_rgba(15,23,42,0.04)] backdrop-blur-sm transition-[border-color,box-shadow,background] hover:border-primary/45 hover:bg-base-100 hover:shadow-[0_0_0_1px_color-mix(in_oklch,var(--color-primary)_28%,transparent),0_2px_6px_rgba(15,23,42,0.06)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40';

export function CreatorNavActions() {
  const { t } = useI18n();
  const [tronCopied, setTronCopied] = useState(false);
  const [emailCopied, setEmailCopied] = useState(false);

  const copyTron = useCallback(async () => {
    const ok = await writeClipboard(TRON_ADDRESS);
    if (!ok) return;
    setEmailCopied(false);
    setTronCopied(true);
    window.setTimeout(() => setTronCopied(false), 2200);
  }, []);

  const copyEmail = useCallback(async () => {
    const ok = await writeClipboard(CONTACT_EMAIL);
    if (!ok) return;
    setTronCopied(false);
    setEmailCopied(true);
    window.setTimeout(() => setEmailCopied(false), 2200);
  }, []);

  return (
    <div className="flex items-center gap-1 sm:gap-1.5">
      <span className="sr-only" aria-live="polite">
        {tronCopied ? t('nav.tronCopied') : emailCopied ? t('nav.emailCopied') : ''}
      </span>
      <button
        type="button"
        className={`${navBtnClass} w-8 min-w-8 justify-center px-0 sm:w-9 sm:min-w-9`}
        aria-label={t('nav.tiktokAria')}
        title={t('nav.tiktokAria')}
        onClick={() => void openExternal(TIKTOK_URL)}
      >
        <TikTokIcon className="h-[22px] w-[22px] drop-shadow-[0_0_10px_color-mix(in_oklch,var(--color-primary)_25%,transparent)]" />
      </button>
      <button
        type="button"
        className={`${navBtnClass} w-8 min-w-8 justify-center px-0 sm:w-9 sm:min-w-9`}
        aria-label={t('nav.contactEmailAria')}
        title={t('nav.contactEmailHover')}
        onClick={() => void copyEmail()}
      >
        {emailCopied ? (
          <CheckMini className="h-[19px] w-[19px] shrink-0 text-success" />
        ) : (
          <MailIcon className="h-[19px] w-[19px] text-base-content/80" />
        )}
      </button>
      <button
        type="button"
        className={`${navBtnClass} max-w-[11rem] sm:max-w-none`}
        onClick={() => void copyTron()}
        aria-label={t('nav.tronSupportAria')}
        title={t('nav.tronSupportHint')}
      >
        {tronCopied ? (
          <CheckMini className="h-4 w-4 shrink-0 text-success" />
        ) : (
          <TronIcon className="h-[18px] w-[18px] shrink-0 drop-shadow-[0_0_8px_color-mix(in_oklch,var(--color-error)_18%,transparent)]" />
        )}
        <span className="hidden max-w-[9rem] truncate text-[10px] font-semibold uppercase tracking-wide text-base-content/80 sm:inline">
          {tronCopied ? t('nav.tronCopied') : 'TRX'}
        </span>
      </button>
    </div>
  );
}
