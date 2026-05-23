import fs from 'fs';

const s = fs.readFileSync('src/i18n/translations.ts', 'utf8');

function extractDeviceInfoBlock(source, startMarker) {
  const start = source.indexOf(startMarker);
  if (start < 0) return {};
  const slice = source.slice(start);
  const end = slice.search(/\n  'brand\.tagline'/);
  const block = end > 0 ? slice.slice(0, end) : slice;
  const re = /'(deviceInfo\.[^']+)':\s*(?:'((?:\\'|[^'])*)'|`([^`]+)`)/g;
  const out = {};
  let m;
  while ((m = re.exec(block))) {
    const val = (m[2] ?? m[3] ?? '').replace(/\\'/g, "'");
    out[m[1]] = val;
  }
  return out;
}

const en = extractDeviceInfoBlock(s, "'deviceInfo.title': 'Device information'");
const fr = extractDeviceInfoBlock(s, "'deviceInfo.title': 'Informations appareil'");

/** PT / ES : base FR atelier + retouches courantes. */
function frToPt(text) {
  return text
    .replace(/Informations appareil/g, 'Informações do dispositivo')
    .replace(
      /Lecture USB locale depuis l’iPhone connecté\. Affichage ici uniquement — rien n’est envoyé en ligne\./g,
      'Leitura USB do iPhone ligado. Mostrado só neste computador — nada é enviado.',
    )
    .replace(/Lecture USB locale/g, 'Leitura USB')
    .replace(/Affichage ici uniquement/g, 'Mostrado só neste computador')
    .replace(/rien n’est envoyé en ligne/g, 'nada é enviado')
    .replace(/rien n’est envoyé/g, 'nada é enviado')
    .replace(
      /IMEI, numéro de série et UDID — quelques lectures USB rapides, affichées uniquement sur cet ordinateur\./g,
      'IMEI, número de série e UDID — leituras USB rápidas, só neste PC.',
    )
    .replace(
      /Touchez le téléphone pour afficher l’IMEI \(si iOS l’expose\), le numéro de série et l’UDID\./g,
      'Toque no telefone para IMEI (se o iOS expuser), número de série e UDID.',
    )
    .replace(
      /Toucher le téléphone pour N° de série et ECID — lecture locale uniquement\./g,
      'Toque no telefone para número de série e ECID — só leitura local.',
    )
    .replace(
      /Valeurs lues en USB dans cet état \(irecovery\)\. UDID et 2ᵉ IMEI non disponibles\./g,
      'Valores lidos por USB neste estado (irecovery). UDID e 2.º IMEI indisponíveis.',
    )
    .replace(
      /Pas de N° de série\. Ajoute irecovery\.exe à côté d’ideviceinfo puis resynchronise\./g,
      'Sem número de série. Coloque irecovery.exe junto a ideviceinfo e sincronize.',
    )
    .replace(
      /Copier les lignes visibles \(séparateur tabulation\)/g,
      'Copiar linhas visíveis (separadas por tabulação)',
    )
    .replace(/Aucune ligne ne correspond au filtre\./g, 'Nenhuma linha corresponde ao filtro.')
    .replace(/lecture locale uniquement/g, 'só leitura local')
    .replace(/numéro de série/g, 'número de série')
    .replace(/Numéro de série/g, 'Número de série')
    .replace(/N° de série/g, 'Número de série')
    .replace(/depuis l’iPhone connecté/g, 'do iPhone ligado')
    .replace(/en ligne/g, '')
    .replace(/ ne /g, ' não ')
    .replace(/Fermer/g, 'Fechar')
    .replace(/Actualiser/g, 'Atualizar')
    .replace(/Copier/g, 'Copiar')
    .replace(/Copié/g, 'Copiado')
    .replace(/Batterie/g, 'Bateria')
    .replace(/Stockage/g, 'Armazenamento')
    .replace(/Inconnu/g, 'Desconhecido')
    .replace(/Oui/g, 'Sim')
    .replace(/Non/g, 'Não')
    .replace(/Détails/g, 'Detalhes')
    .replace(/Redémarrer/g, 'Reiniciar')
    .replace(/Éteindre/g, 'Desligar')
    .replace(/En charge/g, 'A carregar')
    .replace(/Sur batterie/g, 'Na bateria')
    .replace(/Secteur branché/g, 'Na corrente')
    .replace(/Paramètre/g, 'Parâmetro')
    .replace(/Valeur/g, 'Valor')
    .replace(/Filtrer/g, 'Filtrar')
    .replace(/lignes/g, 'linhas')
    .replace(/Tout copier/g, 'Copiar tudo')
    .replace(/Aucune ligne/g, 'Nenhuma linha')
    .replace(/Retour aperçu/g, 'Voltar à visão geral')
    .replace(/Version iOS/g, 'Versão iOS')
    .replace(/N° de série/g, 'Número de série')
    .replace(/Garantie/g, 'Garantia')
    .replace(/Recherche web/g, 'Pesquisa web')
    .replace(/Santé \(rapportée\)/g, 'Saúde (reportada)')
    .replace(/Cycles de charge/g, 'Ciclos de carga')
    .replace(/Disponible/g, 'Disponível')
    .replace(/État USB/g, 'Estado USB')
    .replace(/Lecture…/g, 'A ler…')
    .replace(/Lecture impossible/g, 'Não foi possível ler');
}

function frToEs(text) {
  return text
    .replace(/Informations appareil/g, 'Información del dispositivo')
    .replace(
      /Lecture USB locale depuis l’iPhone connecté\. Affichage ici uniquement — rien n’est envoyé en ligne\./g,
      'Lectura USB del iPhone conectado. Solo en este equipo — no se sube nada.',
    )
    .replace(/Lecture USB locale/g, 'Lectura USB local')
    .replace(/Affichage ici uniquement/g, 'Solo en este equipo')
    .replace(/rien n’est envoyé en ligne/g, 'no se sube nada')
    .replace(/rien n’est envoyé/g, 'no se sube nada')
    .replace(
      /IMEI, numéro de série et UDID — quelques lectures USB rapides, affichées uniquement sur cet ordinateur\./g,
      'IMEI, número de serie y UDID — lecturas USB rápidas, solo en este PC.',
    )
    .replace(
      /Touchez le téléphone pour afficher l’IMEI \(si iOS l’expose\), le numéro de série et l’UDID\./g,
      'Toca el iPhone para IMEI (si iOS lo expone), número de serie y UDID.',
    )
    .replace(
      /Toucher le téléphone pour N° de série et ECID — lecture locale uniquement\./g,
      'Toca el iPhone para número de serie y ECID — solo lectura local.',
    )
    .replace(
      /Valeurs lues en USB dans cet état \(irecovery\)\. UDID et 2ᵉ IMEI non disponibles\./g,
      'Valores leídos por USB en este estado (irecovery). UDID y 2.º IMEI no disponibles.',
    )
    .replace(
      /Pas de N° de série\. Ajoute irecovery\.exe à côté d’ideviceinfo puis resynchronise\./g,
      'Sin número de serie. Añade irecovery.exe junto a ideviceinfo y resincroniza.',
    )
    .replace(
      /Copier les lignes visibles \(séparateur tabulation\)/g,
      'Copiar filas visibles (separadas por tabulación)',
    )
    .replace(/Aucune ligne ne correspond au filtre\./g, 'Ninguna fila coincide con el filtro.')
    .replace(/lecture locale uniquement/g, 'solo lectura local')
    .replace(/numéro de série/g, 'número de serie')
    .replace(/Numéro de série/g, 'Número de serie')
    .replace(/N° de série/g, 'Número de serie')
    .replace(/depuis l’iPhone connecté/g, 'del iPhone conectado')
    .replace(/Fermer/g, 'Cerrar')
    .replace(/Actualiser/g, 'Actualizar')
    .replace(/Copier/g, 'Copiar')
    .replace(/Copié/g, 'Copiado')
    .replace(/Batterie/g, 'Batería')
    .replace(/Stockage/g, 'Almacenamiento')
    .replace(/Inconnu/g, 'Desconocido')
    .replace(/Oui/g, 'Sí')
    .replace(/Non/g, 'No')
    .replace(/Détails/g, 'Detalles')
    .replace(/Redémarrer/g, 'Reiniciar')
    .replace(/Éteindre/g, 'Apagar')
    .replace(/En charge/g, 'Cargando')
    .replace(/Sur batterie/g, 'Con batería')
    .replace(/Secteur branché/g, 'Con corriente')
    .replace(/Paramètre/g, 'Parámetro')
    .replace(/Valeur/g, 'Valor')
    .replace(/Filtrer/g, 'Filtrar')
    .replace(/lignes/g, 'filas')
    .replace(/Tout copier/g, 'Copiar todo')
    .replace(/Aucune ligne/g, 'Ninguna fila')
    .replace(/Retour aperçu/g, 'Volver al resumen')
    .replace(/Version iOS/g, 'Versión iOS')
    .replace(/N° de série/g, 'Número de serie')
    .replace(/Garantie/g, 'Garantía')
    .replace(/Recherche web/g, 'Búsqueda web')
    .replace(/Santé \(rapportée\)/g, 'Estado (informado)')
    .replace(/Cycles de charge/g, 'Ciclos de carga')
    .replace(/Disponible/g, 'Disponible')
    .replace(/État USB/g, 'Estado USB')
    .replace(/Lecture…/g, 'Leyendo…')
    .replace(/Lecture impossible/g, 'No se pudo leer');
}

const pt = {};
const es = {};
for (const [k, v] of Object.entries(fr)) {
  pt[k] = frToPt(v);
  es[k] = frToEs(v);
}

// RU / ZH / AR : surcharges UI principales (le reste hérite EN)
const ruUi = {
  'deviceInfo.title': 'Сведения об устройстве',
  'deviceInfo.subtitle':
    'Чтение по USB с подключённого iPhone. Только на этом ПК — никуда не отправляется.',
  'deviceInfo.idsBadge': 'Lockdown',
  'deviceInfo.idsLead': 'IMEI, серийный номер и UDID — быстрое USB-чтение, только на этом ПК.',
  'deviceInfo.loading': 'Чтение…',
  'deviceInfo.error': 'Не удалось прочитать:',
  'deviceInfo.close': 'Закрыть',
  'deviceInfo.retry': 'Обновить',
  'deviceInfo.copyValue': 'Копировать',
  'deviceInfo.searchPlaceholder': 'Фильтр параметра или значения…',
  'deviceInfo.copyAll': 'Копировать всё',
  'deviceInfo.colParameter': 'Параметр',
  'deviceInfo.colValue': 'Значение',
  'deviceInfo.copiedToast': 'Скопировано.',
  'deviceInfo.dash.charging': 'Зарядка',
  'deviceInfo.dash.onAc': 'От сети',
  'deviceInfo.dash.onBattery': 'От батареи',
  'deviceInfo.dash.restart': 'Перезагрузка',
  'deviceInfo.dash.shutdown': 'Выключить',
  'deviceInfo.dash.refresh': 'Обновить',
  'deviceInfo.dash.viewFull': 'Все поля lockdown',
  'deviceInfo.dash.backOverview': 'К обзору',
  'deviceInfo.dash.yes': 'Да',
  'deviceInfo.dash.no': 'Нет',
  'deviceInfo.dash.unknown': 'Неизвестно',
  'deviceInfo.dash.cardBattery': 'Батарея',
  'deviceInfo.dash.cardStorage': 'Память',
  'deviceInfo.dash.cardVerify': 'Статус USB',
  'deviceInfo.dash.details': 'Подробнее',
};

const zhUi = {
  'deviceInfo.title': '设备信息',
  'deviceInfo.subtitle': '通过 USB 读取已连接的 iPhone。仅在本机显示，不会上传。',
  'deviceInfo.loading': '读取中…',
  'deviceInfo.error': '无法读取：',
  'deviceInfo.close': '关闭',
  'deviceInfo.retry': '刷新',
  'deviceInfo.copyValue': '复制',
  'deviceInfo.searchPlaceholder': '筛选参数或数值…',
  'deviceInfo.copyAll': '全部复制',
  'deviceInfo.colParameter': '参数',
  'deviceInfo.colValue': '数值',
  'deviceInfo.copiedToast': '已复制。',
  'deviceInfo.dash.charging': '充电中',
  'deviceInfo.dash.onAc': '接通电源',
  'deviceInfo.dash.onBattery': '使用电池',
  'deviceInfo.dash.restart': '重新启动',
  'deviceInfo.dash.shutdown': '关机',
  'deviceInfo.dash.refresh': '刷新',
  'deviceInfo.dash.viewFull': '查看全部 lockdown 字段',
  'deviceInfo.dash.backOverview': '返回概览',
  'deviceInfo.dash.yes': '是',
  'deviceInfo.dash.no': '否',
  'deviceInfo.dash.unknown': '未知',
  'deviceInfo.dash.cardBattery': '电池',
  'deviceInfo.dash.cardStorage': '存储',
  'deviceInfo.dash.cardVerify': 'USB 状态',
  'deviceInfo.dash.details': '详情',
};

const arUi = {
  'deviceInfo.title': 'معلومات الجهاز',
  'deviceInfo.subtitle': 'قراءة عبر USB من الآيفون المتصل. تظهر هنا فقط — لا يتم الرفع.',
  'deviceInfo.loading': 'جارٍ القراءة…',
  'deviceInfo.error': 'تعذّرت القراءة:',
  'deviceInfo.close': 'إغلاق',
  'deviceInfo.retry': 'تحديث',
  'deviceInfo.copyValue': 'نسخ',
  'deviceInfo.searchPlaceholder': 'تصفية المعامل أو القيمة…',
  'deviceInfo.copyAll': 'نسخ الكل',
  'deviceInfo.colParameter': 'المعامل',
  'deviceInfo.colValue': 'القيمة',
  'deviceInfo.copiedToast': 'تم النسخ.',
  'deviceInfo.dash.charging': 'يشحن',
  'deviceInfo.dash.onAc': 'على التيار',
  'deviceInfo.dash.onBattery': 'على البطارية',
  'deviceInfo.dash.restart': 'إعادة التشغيل',
  'deviceInfo.dash.shutdown': 'إيقاف التشغيل',
  'deviceInfo.dash.refresh': 'تحديث',
  'deviceInfo.dash.viewFull': 'كل حقول lockdown',
  'deviceInfo.dash.backOverview': 'العودة للنظرة العامة',
  'deviceInfo.dash.yes': 'نعم',
  'deviceInfo.dash.no': 'لا',
  'deviceInfo.dash.unknown': 'غير معروف',
  'deviceInfo.dash.cardBattery': 'البطارية',
  'deviceInfo.dash.cardStorage': 'التخزين',
  'deviceInfo.dash.cardVerify': 'حالة USB',
  'deviceInfo.dash.details': 'تفاصيل',
};

const out = `import type { Messages } from './translations';

/** Surcharges deviceInfo.* — clés absentes héritent de \`en\` via translateKey. */
export const DEVICE_INFO_LOCALES: Record<string, Messages> = {
  pt: ${JSON.stringify(pt, null, 2)} as Messages,
  es: ${JSON.stringify(es, null, 2)} as Messages,
  ru: ${JSON.stringify(ruUi, null, 2)} as Messages,
  zh: ${JSON.stringify(zhUi, null, 2)} as Messages,
  ar: ${JSON.stringify(arUi, null, 2)} as Messages,
};
`;

fs.writeFileSync('src/i18n/localeDeviceInfo.ts', out);
console.log('deviceInfo EN keys:', Object.keys(en).length);
console.log('deviceInfo FR keys:', Object.keys(fr).length);
console.log('pt/es overrides:', Object.keys(pt).length);
