/**
 * Signal sonore courte connexion USB — léger motif ascendant façon toast produit
 * récent (empilements doux, pas un double « dong » tonal cheap).
 *
 * Synthèse locale Web Audio uniquement (aucun fichier, pas d’extrait système tiers).
 */
let sharedCtx: AudioContext | null = null;

function getAudioContext(): AudioContext | null {
  const Ctor = window.AudioContext ?? (window as Window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctor) return null;
  if (!sharedCtx || sharedCtx.state === 'closed') {
    sharedCtx = new Ctor();
  }
  return sharedCtx;
}

export async function playConnectChime(): Promise<void> {
  try {
    const ctx = getAudioContext();
    if (!ctx) return;

    await ctx.resume().catch(() => {});

    const t0 = ctx.currentTime;

    /* Bus : compression légère + master pour volume global */
    const compressor = ctx.createDynamicsCompressor();
    compressor.threshold.value = -22;
    compressor.knee.value = 14;
    compressor.ratio.value = 3;
    compressor.attack.value = 0.003;
    compressor.release.value = 0.12;

    const master = ctx.createGain();
    master.gain.setValueAtTime(0.085, t0);

    const lowpass = ctx.createBiquadFilter();
    lowpass.type = 'lowpass';
    lowpass.frequency.setValueAtTime(6200, t0);
    lowpass.Q.setValueAtTime(0.55, t0);

    master.connect(lowpass).connect(compressor).connect(ctx.destination);

    /** Triade majeure courte (Mi–Sol–Si) léger léger mouillage stéréophonique perceptif via deux voix désaccordées. */
    type Note = {
      freqBase: number;
      start: number;
      duration: number;
      peakRel: number;
      oscType: OscillatorType;
    };

    const notes: Note[] = [
      { freqBase: 659.25, start: 0, duration: 0.22, peakRel: 0.55, oscType: 'triangle' }, // Mi5 — attaque douce
      { freqBase: 783.99, start: 0.05, duration: 0.2, peakRel: 0.42, oscType: 'triangle' }, // Sol5
      { freqBase: 987.77, start: 0.1, duration: 0.26, peakRel: 0.34, oscType: 'sine' }, // Si5 — brillance discrète
    ];

    for (const n of notes) {
      const cents = [-2.8, 2.8] as const;
      for (const centsOff of cents) {
        const detune = centsOff;

        const osc = ctx.createOscillator();
        osc.type = n.oscType;
        osc.detune.value = detune;
        osc.frequency.setValueAtTime(n.freqBase, t0 + n.start);

        const gn = ctx.createGain();
        const startWall = n.start + 0.01;
        const attack = Math.min(0.022, n.duration * 0.33);
        const end = Math.max(startWall + n.duration, startWall + 0.055);
        gn.gain.setValueAtTime(1e-4, t0 + n.start);

        gn.gain.exponentialRampToValueAtTime(n.peakRel, t0 + n.start + attack);
        gn.gain.exponentialRampToValueAtTime(1e-4, t0 + end);

        osc.connect(gn).connect(master);
        osc.start(t0 + n.start);
        osc.stop(t0 + end + 0.04);
      }
    }

    /* Petit « clic » amorti bande médium — effet tactile propre sans grincer */
    const clickDur = 0.028;
    const noiseBuf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * clickDur), ctx.sampleRate);
    const cd = noiseBuf.getChannelData(0);
    for (let i = 0; i < cd.length; i++) cd[i] = (Math.random() * 2 - 1) * Math.exp(-(i / cd.length) * 5);

    const ns = ctx.createBufferSource();
    ns.buffer = noiseBuf;

    const bp = ctx.createBiquadFilter();
    bp.type = 'bandpass';
    bp.frequency.setValueAtTime(2600, t0);
    bp.Q.setValueAtTime(0.88, t0);

    const ng = ctx.createGain();
    ng.gain.setValueAtTime(4e-4, t0);
    ng.gain.exponentialRampToValueAtTime(0.072, t0 + 0.004);
    ng.gain.exponentialRampToValueAtTime(1e-4, t0 + clickDur);

    ns.connect(bp).connect(ng).connect(master);
    ns.start(t0);
    ns.stop(t0 + clickDur + 0.01);

    window.setTimeout(() => {
      try {
        ns.disconnect();
        master.disconnect();
        lowpass.disconnect();
        compressor.disconnect();
      } catch {
        /* noop */
      }
    }, 520);
  } catch {
    /* autoplay peut rester suspendu avant une interaction utilisateur */
  }
}
