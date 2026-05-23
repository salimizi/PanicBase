import { useI18n } from '../i18n/context';

/** Titre + baseline marque + ligne produit (navbar) */

const APP_VERSION = '0.2';

export function BrandHeader() {
  const { t } = useI18n();
  const tagline = t('brand.tagline');
  const [taglineMain, taglineRest] = tagline.includes(' · ')
    ? (tagline.split(' · ', 2) as [string, string])
    : [tagline, ''];

  return (
    <div className="brand-lockup flex min-w-0 flex-col items-start gap-0.5 text-left">
      <div className="flex min-w-0 flex-row flex-wrap items-baseline gap-x-2 gap-y-0.5">
        <span
          className="inline-flex min-w-0 items-baseline font-syne text-[clamp(1.08rem,3.1vw,1.32rem)] font-bold tracking-[-0.02em]"
          dir="ltr"
          aria-current="page"
        >
          <span className="brand-word-panic">Panic</span>
          <span className="brand-word-base">Base</span>
        </span>
        <span className="badge badge-sm shrink-0 border-0 bg-base-200/70 font-sora text-[10px] font-medium normal-case tracking-normal text-base-content/50 dark:bg-white/[0.08]">
          {APP_VERSION}
        </span>
      </div>
      <p className="brand-baseline font-sora m-0 max-w-[min(100%,26rem)] text-[9px] font-semibold leading-tight tracking-[0.14em]">
        {t('brand.baseline')}
      </p>
      <p className="brand-tagline font-sora m-0 max-w-[24rem] text-[11px] font-normal leading-relaxed tracking-normal text-base-content/55 sm:text-[11.5px]">
        {taglineRest ? (
          <>
            {taglineMain}
            <span className="text-base-content/45"> · </span>
            <span className="tracking-[0.02em] text-base-content/75">{taglineRest}</span>
          </>
        ) : (
          taglineMain
        )}
      </p>
    </div>
  );
}
