import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import {
  defaultLocale,
  LOCALE_STORAGE_KEY,
  readStoredLocale,
  translateKey,
  type Locale,
} from './translations';

type Ctx = {
  locale: Locale;
  setLocale: (l: Locale) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
};

const I18nContext = createContext<Ctx | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() =>
    typeof window !== 'undefined' ? readStoredLocale() : defaultLocale(),
  );

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    try {
      localStorage.setItem(LOCALE_STORAGE_KEY, l);
    } catch {
      /* ignore */
    }
    document.documentElement.lang = l;
    document.documentElement.dir = 'ltr';
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dir = 'ltr';
  }, [locale]);

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>) => translateKey(locale, key, vars),
    [locale],
  );

  const value = useMemo(() => ({ locale, setLocale, t }), [locale, setLocale, t]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): Ctx {
  const v = useContext(I18nContext);
  if (!v) throw new Error('useI18n must be used within I18nProvider');
  return v;
}

/** Pour composants qui peuvent s’afficher sans arbre `I18nProvider` (ex. double React en dev). */
export function useOptionalI18n(): Ctx | null {
  return useContext(I18nContext);
}
