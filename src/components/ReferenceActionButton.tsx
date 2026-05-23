import type { ButtonHTMLAttributes, ReactNode } from 'react';

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
  /** Style « primaire » façon accent #4f8ef7 de la fiche HTML */
  accent?: boolean;
};

export function ReferenceActionButton({ accent, className = '', children, type = 'button', ...rest }: Props) {
  const base =
    'inline-flex shrink-0 items-center justify-center rounded-md border px-2.5 py-1.5 font-mono text-[11px] font-medium leading-none transition-colors disabled:cursor-not-allowed disabled:opacity-40';
  const theme = accent
    ? 'border-[rgba(79,142,247,0.45)] bg-[rgba(79,142,247,0.14)] text-[#93bffc] hover:bg-[rgba(79,142,247,0.24)]'
    : 'border-[#2a2a30] bg-[#1a1a1e] text-[#a0a0b4] hover:border-[#3f3f48] hover:bg-[#222226] hover:text-[#e8e8f0]';
  return (
    <button type={type} className={`${base} ${theme} ${className}`.trim()} {...rest}>
      {children}
    </button>
  );
}
