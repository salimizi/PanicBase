import { useI18n } from '../i18n/context';
import type { UsbPhase } from './usbTypes';
import { AppleMark } from './AppleMark';

export type { UsbPhase };

/** Même fond photo pour hub connecté et mode recovery (lisibilité cohérente). */
export const HERO_IPHONE_WALLPAPER_SRC = 'https://img.daisyui.com/images/stock/453966.webp';

type Props = {
  phase: UsbPhase | string;
  /** Agrandit le dock (écran d’accueil centré) */
  hero?: boolean;
  /** Affiché sur l’écran du téléphone (phase connectée) */
  screenTitle?: string | null;
  screenSubtitle?: string | null;
  /** Phase connectée + hero : clic / clavier sur le mockup (détails appareil) */
  onConnectedFrameClick?: () => void;
};

export function PhoneUsbDock({ phase, hero, screenTitle, screenSubtitle, onConnectedFrameClick }: Props) {
  const { t } = useI18n();
  /** Variante visuelle du dock (cadre + câble) — recovery ≠ connecté (teinte recovery). */
  const d =
    phase === 'no_tools' || phase === 'error'
      ? 'unplugged'
      : phase === 'awaiting_trust'
        ? 'trust'
        : phase === 'connected'
          ? 'connected'
          : phase === 'recovery'
            ? 'recovery'
            : 'plug';

  const heroClass = hero ? 'phone-dock--hero phone-dock--solo' : '';

  const mockupToneClass =
    d === 'connected' || d === 'recovery'
      ? 'mockup-tone-connected'
      : d === 'trust'
        ? 'mockup-tone-trust'
        : d === 'plug'
          ? 'mockup-tone-plug'
          : 'mockup-tone-muted';

  const auroraToneClass =
    d === 'connected'
      ? 'mockup-display-aurora mockup-display-aurora--connected'
      : d === 'recovery'
        ? 'mockup-display-aurora mockup-display-aurora--recovery'
        : d === 'trust'
          ? 'mockup-display-aurora mockup-display-aurora--trust'
          : d === 'plug'
            ? 'mockup-display-aurora mockup-display-aurora--plug'
            : 'mockup-display-aurora mockup-display-aurora--muted';

  const a11yInteractive =
    (phase === 'connected' || phase === 'recovery') && Boolean(onConnectedFrameClick);

  const phoneFrame = (
    <div
      className={`mockup-phone dock-phone border-[#ff8938] ${mockupToneClass}${
        a11yInteractive
          ? ' cursor-pointer transition-[box-shadow] hover:ring-2 hover:ring-primary/35 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary'
          : ''
      }`.trim()}
      onClick={a11yInteractive ? onConnectedFrameClick : undefined}
      onKeyDown={
        a11yInteractive
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onConnectedFrameClick?.();
              }
            }
          : undefined
      }
      role={a11yInteractive ? 'button' : undefined}
      tabIndex={a11yInteractive ? 0 : undefined}
    >
      <div className="mockup-phone-camera" />
      <div
        className={
          d === 'trust'
            ? 'mockup-phone-display phone-display-stack phone-display-stack--trust'
            : 'mockup-phone-display phone-display-stack'
        }
      >
        <img
          alt=""
          className={d === 'trust' ? 'mockup-phone-wallpaper mockup-phone-wallpaper--trust' : 'mockup-phone-wallpaper'}
          src={HERO_IPHONE_WALLPAPER_SRC}
          draggable={false}
          loading="lazy"
        />
        <div className={auroraToneClass} aria-hidden />
        <div
          className={
            d === 'connected' || d === 'recovery'
              ? 'phone-display-content phone-display-content--connected-hub'
              : d === 'trust'
                ? 'phone-display-content phone-display-content--trust-sheet'
                : 'phone-display-content'
          }
        >
          {d === 'trust' ? (
            <div
              className="ios-trust-sheet-ios"
              role="dialog"
              aria-label={t('usb.trustSheetAria')}
              aria-live="polite"
            >
              <p className="ios-trust-title">{t('usb.trustTitle')}</p>
              <p className="ios-trust-body">{t('usb.trustBody')}</p>
              <div className="ios-trust-actions" aria-hidden="true">
                <span className="ios-trust-btn ios-trust-btn-decline">{t('usb.trustDecline')}</span>
                <span className="ios-trust-btn ios-trust-btn-accept">{t('usb.trustAccept')}</span>
              </div>
            </div>
          ) : null}
          {d === 'connected' ? (
            <div className="connected-specs connected-specs--hub">
              <div className="connected-apple-slot">
                <span className="connected-apple-halo" aria-hidden />
                <AppleMark className="apple-mark-hub" />
              </div>
              <span className="spec-main spec-main--hub">{screenTitle ?? '—'}</span>
              {screenSubtitle ? <span className="spec-ios spec-ios--hub">{screenSubtitle}</span> : null}
            </div>
          ) : null}
          {d === 'recovery' ? (
            <div className="connected-specs connected-specs--hub connected-specs--recovery">
              <div className="connected-apple-slot connected-apple-slot--recovery">
                <span className="connected-apple-halo connected-apple-halo--recovery" aria-hidden />
                <AppleMark className="apple-mark-hub apple-mark-hub--recovery" />
              </div>
              <span className="spec-main spec-main--hub spec-main--recovery">{screenTitle ?? '—'}</span>
              {screenSubtitle ? (
                <span className="spec-ios spec-ios--hub spec-ios--recovery">{screenSubtitle}</span>
              ) : null}
            </div>
          ) : null}
          {d === 'plug' ? (
            <div className="plug-hint-mini plug-hint-mini--solo">
              <span className="plug-glow-ring" />
              <span className="plug-dot" />
              <span className="plug-caption">USB</span>
            </div>
          ) : null}
        </div>
      </div>
      {d === 'connected' ? (
        <>
          <span className="phone-frame-shimmer-overlay" aria-hidden />
          <span className="phone-frame-sparkle" aria-hidden />
        </>
      ) : null}
      {d === 'recovery' ? (
        <>
          <span className="phone-frame-shimmer-overlay phone-frame-shimmer-overlay--recovery" aria-hidden />
          <span className="phone-frame-sparkle phone-frame-sparkle--recovery" aria-hidden />
        </>
      ) : null}
    </div>
  );

  return (
    <div className={`phone-dock dock--${d} ${heroClass}`.trim()} aria-hidden={!a11yInteractive}>
      {!hero ? (
        <>
          <div className="dock-laptop-edge" />
          <div className="dock-cable-wrap">
            <div className="dock-cable" />
            <div className="dock-connector" />
          </div>
        </>
      ) : null}
      {hero ? phoneFrame : <div className="dock-mockup-compact">{phoneFrame}</div>}
    </div>
  );
}
