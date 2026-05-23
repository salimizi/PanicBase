import { useMemo, useState } from 'react';
import { useI18n } from '../i18n/context';

export type PanicLogRow = {
  index: number;
  filename: string;
  modifiedLabel: string;
  snippet: string;
};

type Props = {
  logs: PanicLogRow[];
  selectedIndex: number | null;
  onSelect: (index: number) => void;
  disabled?: boolean;
  /** Panneau 50/50 (iPhone connecté) : zone de liste plus haute */
  expanded?: boolean;
};

function normalize(s: string) {
  return s.toLowerCase().normalize('NFD').replace(/\p{M}/gu, '');
}

function rowMatches(row: PanicLogRow, q: string): boolean {
  const t = q.trim();
  if (!t) return true;
  const n = normalize(t);
  const hay = normalize(`${row.filename} ${row.modifiedLabel} ${row.snippet}`);
  return hay.includes(n);
}

export function PanicLogsTable({ logs, selectedIndex, onSelect, disabled, expanded }: Props) {
  const { t } = useI18n();
  const [query, setQuery] = useState('');

  const filtered = useMemo(() => logs.filter((r) => rowMatches(r, query)), [logs, query]);

  return (
    <div className={`flex min-h-0 flex-col gap-2 ${expanded ? 'flex-1' : ''}`}>
      <label className="input input-bordered input-sm flex w-full items-center gap-2 border-base-content/15 bg-base-200/40">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="h-3.5 w-3.5 shrink-0 opacity-50"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
          aria-hidden
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="search"
          className="grow font-sora text-xs placeholder:opacity-50"
          placeholder={t('table.search')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          disabled={disabled}
          autoComplete="off"
          spellCheck={false}
        />
        {query.trim() ? (
          <span className="badge badge-ghost badge-xs shrink-0 font-mono">
            {filtered.length}/{logs.length}
          </span>
        ) : null}
      </label>

      {filtered.length === 0 ? (
        <p className="m-0 rounded-lg border border-base-content/10 bg-base-200/30 py-6 text-center text-xs text-base-content/55">
          {t('table.noMatch')}
        </p>
      ) : (
        <div
          className={`pb-log-table-wrap min-h-0 overflow-x-auto overflow-y-auto ${
            expanded ? 'max-h-[min(68vh,600px)] min-h-[12rem]' : 'max-h-[min(38vh,280px)]'
          }`}
        >
          <table className="table table-pin-rows table-sm w-full">
            <thead>
              <tr className="border-b border-[#2a2a30] font-sora text-[10px] uppercase tracking-wider text-[#8888a0]">
                <th className="w-8 bg-[#16161a]">{t('table.colHash')}</th>
                <th className="min-w-[7rem] bg-[#16161a]">{t('table.colFile')}</th>
                <th className="w-[4.5rem] whitespace-nowrap bg-[#16161a]">{t('table.colModified')}</th>
                <th className="hidden min-w-[6rem] sm:table-cell bg-[#16161a]">{t('table.colSnippet')}</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((row) => {
                const sel = selectedIndex === row.index;
                return (
                  <tr
                    key={`${row.filename}-${row.index}`}
                    className={`cursor-pointer border-b border-[#25252d] font-sora text-xs text-[#cfcfd8] transition-colors hover:bg-white/[0.04] [&:nth-child(even)]:bg-white/[0.02] ${
                      sel ? 'pb-log-row-active' : ''
                    }`}
                    onClick={() => {
                      if (!disabled) onSelect(row.index);
                    }}
                    onKeyDown={(e) => {
                      if (disabled) return;
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        onSelect(row.index);
                      }
                    }}
                    tabIndex={disabled ? -1 : 0}
                    role="button"
                    aria-pressed={sel}
                    aria-label={t('table.openRow').replace('{{file}}', row.filename)}
                  >
                    <td className="font-mono text-[10px] font-bold text-base-content/70">{row.index + 1}</td>
                    <td className="max-w-[1px]">
                      <span className="block truncate font-mono font-semibold text-base-content" title={row.filename}>
                        {row.filename}
                      </span>
                    </td>
                    <td className="whitespace-nowrap text-[10px] font-medium text-base-content/60">{row.modifiedLabel}</td>
                    <td className="hidden max-w-[1px] sm:table-cell">
                      <span className="line-clamp-2 text-[10px] leading-snug text-base-content/55" title={row.snippet}>
                        {row.snippet || '—'}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
