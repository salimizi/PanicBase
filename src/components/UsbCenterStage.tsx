import { useI18n } from '../i18n/context';
import { PhoneUsbDock } from './PhoneUsbDock';
import type { UsbPhase } from './usbTypes';

type Props = {
  phase: UsbPhase | string;
  /** Ex. « 17 Pro Max » (sans préfixe iPhone) — phase connectée */
  modelShort: string | null;
  /** Ex. « iPhone 17 Pro » — phase recovery (titre complet) */
  recoveryModelFull: string | null;
  /** Sous-titre optionnel sur le mockup recovery (ex. DFU), sans répéter « recovery » */
  recoveryTechLine: string | null;
  iosVersion: string | null;
  /** Message d’erreur / outils (phases error, no_tools) */
  backendDetail?: string;
  /** Incrémenté à chaque entrée dans l’état connecté → relances animations / halo */
  connectCelebrationKey?: number;
  /** iPhone connecté : ouvre le panneau infos (SN, UDID, batterie…) */
  onConnectedPhoneClick?: () => void;
  /** Sortie recovery : irecovery -c fsboot/go */
  onExitRecoveryBoot?: () => void;
  exitRecoveryBootBusy?: boolean;
};

function HeroPhoneWrap({
  phase,
  muted,
  screenTitle,
  screenSubtitle,
  dockKey,
  onConnectedFrameClick,
}: {
  phase: UsbPhase | string;
  muted?: boolean;
  screenTitle?: string | null;
  screenSubtitle?: string | null;
  dockKey?: number;
  onConnectedFrameClick?: () => void;
}) {
  return (
    <div className={`hero-phone-wrap ${muted ? 'hero-phone-wrap--muted' : ''}`}>
      <PhoneUsbDock
        key={`dock-${dockKey ?? 0}`}
        phase={phase}
        hero
        screenTitle={screenTitle}
        screenSubtitle={screenSubtitle}
        onConnectedFrameClick={onConnectedFrameClick}
      />
    </div>
  );
}

export function UsbCenterStage({
  phase,
  modelShort,
  recoveryModelFull,
  recoveryTechLine,
  iosVersion,
  backendDetail,
  connectCelebrationKey = 0,
  onConnectedPhoneClick,
  onExitRecoveryBoot,
  exitRecoveryBootBusy = false,
}: Props) {
  const { t } = useI18n();
  const p = phase as UsbPhase;
  const once = t('usb.inRecoveryOnce');

  if (p === 'unplugged') {
    return (
      <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 px-2 py-2 text-center">
        <HeroPhoneWrap phase="unplugged" dockKey={0} />
        <div className="flex max-w-[min(52ch,100%)] flex-col items-center gap-1">
          <p className="font-sora m-0 text-balance text-xl font-extrabold tracking-tight text-base-content">
            {t('usb.plugCable')}
          </p>
          <p className="m-0 text-balance text-[13px] font-medium leading-snug text-base-content/64 sm:text-sm">
            {t('usb.plugCableHint')}
          </p>
        </div>
        {backendDetail?.trim() ? (
          <p className="m-0 mt-0.5 max-w-[min(52ch,100%)] text-xs leading-snug text-base-content/50">{backendDetail}</p>
        ) : null}
      </div>
    );
  }

  if (p === 'no_tools' || p === 'error') {
    return (
      <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 px-2 py-2 text-center">
        <HeroPhoneWrap phase={p === 'no_tools' ? 'no_tools' : 'error'} muted dockKey={0} />
        <div className="flex max-w-[min(52ch,100%)] flex-col items-center gap-1">
          <p className="font-sora m-0 text-balance text-xl font-extrabold tracking-tight text-base-content">
            {t('usb.plugCable')}
          </p>
          <p className="m-0 text-balance text-[13px] font-medium leading-snug text-base-content/64 sm:text-sm">
            {t('usb.plugCableHint')}
          </p>
        </div>
      </div>
    );
  }

  if (p === 'awaiting_trust') {
    const trustCopy = backendDetail?.trim() ? backendDetail : t('usb.trustDefault');
    return (
      <div className="flex min-h-0 w-full min-w-0 shrink-0 flex-col items-stretch justify-center gap-2 pb-1">
        <div className="flex w-full justify-center">
          <div className="trust-aura">
            <HeroPhoneWrap phase="awaiting_trust" dockKey={0} />
          </div>
        </div>
        <p className="font-sora m-0 w-full min-w-0 hyphens-auto whitespace-normal text-balance text-center text-[13px] font-medium leading-snug tracking-tight text-base-content/82 sm:text-[14px] sm:leading-relaxed">
          {trustCopy}
        </p>
      </div>
    );
  }

  if (p === 'connected') {
    const titleOnScreen = modelShort ?? '?';
    const sub = iosVersion ? `iOS ${iosVersion}` : null;
    const labelId = `device-connected-label-${connectCelebrationKey}`;
    const srIos = sub ? `, ${sub}` : '';
    const recognizedSr = t('usb.recognizedSr', { model: titleOnScreen, ios: srIos });
    return (
      <div
        className="relative flex min-h-0 shrink-0 flex-col items-center justify-center gap-1 px-2 pb-1 pt-2 text-center"
        key={`celebrate-${connectCelebrationKey}`}
        aria-labelledby={labelId}
      >
        <div className="hero-phone-connected-orbit hero-phone-connected-orbit--burst">
          <HeroPhoneWrap
            phase="connected"
            dockKey={connectCelebrationKey}
            screenTitle={titleOnScreen}
            screenSubtitle={sub}
            onConnectedFrameClick={onConnectedPhoneClick}
          />
        </div>
        <span id={labelId} className="sr-only">
          {recognizedSr}
        </span>
        <p
          key={`model-${connectCelebrationKey}-${titleOnScreen}`}
          className="connected-reveal-model font-sora m-0 mt-2 max-w-[min(420px,100%)] px-1 text-[1.45rem] leading-[1.12] font-semibold tracking-[0.02em] sm:text-[1.6rem]"
        >
          <span className="connected-device-line block text-primary">
            {modelShort ?? 'iPhone'}
          </span>
        </p>
        {sub ? (
          <p key={`ios-${connectCelebrationKey}`} className="connected-reveal-ios font-sora ios-version-pill m-0 mt-1.5 font-semibold">
            <span className="connected-ios-brand">iOS</span>
            <span className="connected-ios-numbers tabular-nums">{iosVersion}</span>
          </p>
        ) : (
          <p className="connected-reveal-ios font-sora m-0 mt-1.5 text-[11px] font-medium tracking-[0.24em] text-base-content/42">
            {t('usb.iosDash')}
          </p>
        )}
        <span className="font-sora m-0 mt-1 text-[10px] font-semibold uppercase tracking-[0.48em] text-base-content/42">
          {t('usb.connectedBadge')}
        </span>
        {onConnectedPhoneClick ? (
          <p className="font-sora m-0 mt-1 max-w-[min(44ch,100%)] text-center text-[10px] leading-snug text-base-content/48">
            {t('deviceInfo.tapHint')}
          </p>
        ) : null}
      </div>
    );
  }

  if (p === 'recovery') {
    const heading = recoveryModelFull?.trim() || modelShort || 'iPhone';
    const phoneSub = recoveryTechLine?.trim() || null;
    const labelId = `device-recovery-label-${connectCelebrationKey}`;
    return (
      <div
        className="relative flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 px-2 pb-1 pt-2 text-center"
        key={`recovery-${connectCelebrationKey}`}
        aria-labelledby={labelId}
      >
        <div className="hero-phone-connected-orbit">
          <HeroPhoneWrap
            phase="recovery"
            dockKey={connectCelebrationKey}
            screenTitle={heading}
            screenSubtitle={phoneSub}
            onConnectedFrameClick={onConnectedPhoneClick}
          />
        </div>
        <span id={labelId} className="sr-only">
          {t('usb.recoverySr', { model: heading, once })}
        </span>
        <p className="connected-reveal-model font-sora m-0 mt-2 max-w-[min(420px,100%)] px-1 text-[1.35rem] leading-[1.12] font-semibold tracking-[0.02em] sm:text-[1.5rem]">
          <span className="connected-device-line block text-warning">{heading}</span>
        </p>
        <p className="font-sora m-0 mt-1 text-[12px] font-medium leading-snug text-warning/90">{once}</p>
        {onExitRecoveryBoot ? (
          <button
            type="button"
            className="btn btn-sm btn-warning btn-outline mt-1.5 max-w-[min(280px,100%)] font-sora font-semibold"
            disabled={exitRecoveryBootBusy}
            onClick={onExitRecoveryBoot}
          >
            {exitRecoveryBootBusy ? t('usb.exitRecoveryBootBusy') : t('usb.exitRecoveryBoot')}
          </button>
        ) : null}
        {onConnectedPhoneClick ? (
          <p className="font-sora m-0 mt-1 max-w-[min(44ch,100%)] text-center text-[10px] leading-snug text-base-content/48">
            {t('deviceInfo.recoveryTapHint')}
          </p>
        ) : null}
      </div>
    );
  }

  return (
    <div className="flex min-h-0 shrink-0 flex-col items-center justify-center gap-1.5 px-2 py-2 text-center">
      <HeroPhoneWrap phase="unplugged" dockKey={0} />
      <p className="m-0 text-base-content/50">…</p>
    </div>
  );
}
