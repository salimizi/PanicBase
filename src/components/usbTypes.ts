export type UsbPhase =
  | 'unplugged'
  | 'awaiting_trust'
  | 'connected'
  | 'recovery'
  | 'no_tools'
  | 'error';
