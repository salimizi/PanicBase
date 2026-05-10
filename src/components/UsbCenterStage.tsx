import { PhoneUsbDock } from './PhoneUsbDock';
import type { UsbPhase } from './usbTypes';

type Props = {
  phase: UsbPhase | string;
  /** Ex. « 17 Pro Max » (sans préfixe iPhone) */
  modelShort: string | null;
  iosVersion: string | null;
  /** Message d’erreur / outils (phases error, no_tools) */
  backendDetail?: string;
};

function HeroPhoneWrap({
  phase,
  muted,
  screenTitle,
  screenSubtitle,
}: {
  phase: UsbPhase | string;
  muted?: boolean;
  screenTitle?: string | null;
  screenSubtitle?: string | null;
}) {
  return (
    <div className={`hero-phone-wrap ${muted ? 'hero-phone-wrap--muted' : ''}`}>
      <PhoneUsbDock phase={phase} hero screenTitle={screenTitle} screenSubtitle={screenSubtitle} />
    </div>
  );
}

export function UsbCenterStage({ phase, modelShort, iosVersion, backendDetail }: Props) {
  const p = phase as UsbPhase;

  if (p === 'unplugged') {
    return (
      <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 px-2 py-2 text-center">
        <HeroPhoneWrap phase="unplugged" />
        <p className="m-0 text-xl font-extrabold tracking-tight text-base-content">Brancher le câble USB</p>
      </div>
    );
  }

  if (p === 'no_tools' || p === 'error') {
    return (
      <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 px-2 py-2 text-center">
        <HeroPhoneWrap phase={p === 'no_tools' ? 'no_tools' : 'error'} muted />
        <p className="m-0 text-lg font-extrabold tracking-tight text-error">Problème USB / outils</p>
        {backendDetail ? (
          <p className="m-0 mt-1.5 max-w-[48ch] text-xs leading-snug text-base-content/60">{backendDetail}</p>
        ) : null}
      </div>
    );
  }

  if (p === 'awaiting_trust') {
    return (
      <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 pb-1 text-center">
        <div className="trust-aura">
          <HeroPhoneWrap phase="awaiting_trust" />
        </div>
        <p className="m-0 text-xl font-extrabold tracking-tight text-base-content">Faire confiance à cet ordinateur</p>
        <p className="m-0 -mt-0.5 text-sm font-bold text-warning">Sur l’iPhone · Confiance</p>
      </div>
    );
  }

  if (p === 'connected') {
    const titleOnScreen = modelShort ?? '?';
    const sub = iosVersion ? `iOS ${iosVersion}` : null;
    return (
      <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-0.5 px-2 pb-1 pt-2 text-center">
        <HeroPhoneWrap phase="connected" screenTitle={titleOnScreen} screenSubtitle={sub} />
        <p className="m-0 mt-1.5 bg-gradient-to-br from-base-100 via-info/90 to-info bg-clip-text text-2xl font-black tracking-tighter text-transparent">
          {titleOnScreen}
        </p>
        {sub ? <p className="m-0 mt-0.5 text-sm font-extrabold text-success">{sub}</p> : null}
      </div>
    );
  }

  return (
    <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 px-2 py-2 text-center">
      <HeroPhoneWrap phase="unplugged" />
      <p className="m-0 text-base-content/50">…</p>
    </div>
  );
}
