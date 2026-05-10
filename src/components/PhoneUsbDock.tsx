import type { UsbPhase } from './usbTypes';

export type { UsbPhase };

type Props = {
  phase: UsbPhase | string;
  /** Agrandit le dock (écran d’accueil centré) */
  hero?: boolean;
  /** Affiché sur l’écran du téléphone (phase connectée) */
  screenTitle?: string | null;
  screenSubtitle?: string | null;
};

export function PhoneUsbDock({ phase, hero, screenTitle, screenSubtitle }: Props) {
  const p =
    phase === 'no_tools' || phase === 'error'
      ? 'unplugged'
      : phase === 'awaiting_trust'
        ? 'trust'
        : phase === 'connected'
          ? 'connected'
          : 'plug';

  const heroClass = hero ? 'phone-dock--hero phone-dock--solo' : '';

  return (
    <div className={`phone-dock dock--${p} ${heroClass}`.trim()} aria-hidden>
      {!hero ? (
        <>
          <div className="dock-laptop-edge" />
          <div className="dock-cable-wrap">
            <div className="dock-cable" />
            <div className="dock-connector" />
          </div>
        </>
      ) : null}
      <div className="dock-phone">
        <div className="phone-screen">
          <div className="phone-notch" />
          {p === 'trust' ? (
            <div className="trust-pop trust-pop--hero">
              <span className="trust-ring" />
              <span className="trust-title">Ordinateur</span>
              <span className="trust-yes pulse">Confiance</span>
              <small className="trust-hint">iPhone</small>
            </div>
          ) : null}
          {p === 'connected' ? (
            <div className="connected-specs">
              <span className="check-mini">✓</span>
              <span className="spec-main">{screenTitle ?? '—'}</span>
              {screenSubtitle ? <span className="spec-ios">{screenSubtitle}</span> : null}
            </div>
          ) : null}
          {p === 'plug' ? (
            <div className="plug-hint-mini plug-hint-mini--solo">
              <span className="plug-glow-ring" />
              <span className="plug-dot" />
              <span className="plug-caption">USB</span>
            </div>
          ) : null}
        </div>
        <span className="phone-port-slot" />
      </div>
    </div>
  );
}
