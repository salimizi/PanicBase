Ce dossier est volontairement vide dans Git (licence / poids du dépôt).

Pour embarquer les mêmes binaires que iDevice Panic Log Analyzer (idevice_id.exe, ideviceinfo.exe, irecovery.exe recommandé pour le mode Recovery, *.dll) dans l’installeur PanicBase :

  1) Installer iDevice Panic Log Analyzer (Windows), OU placer idevice_id.exe + DLL dans un dossier local.
  2) Depuis la racine du repo :
       npm run sync:libimobiledevice
     Variante (dossier perso au lieu d’IDA) :
       set PANICBASE_SYNC_IDEVICE_SRC=C:\chemin\vers\dossier_avec_idevice_id
       npm run sync:libimobiledevice
  2b) Si win-x64 ne contient pas irecovery.exe (mode Recovery), ajouter les binaires MSYS2 épinglés :
       npm run fetch:irecovery
       (télécharge depuis mirror.msys2.org, vérifie SHA256, copie irecovery.exe + DLL dans ce dossier.)
  3) Lancer l’app avec le backend Tauri (USB ne marche pas avec « npm run dev » seul) :
       npm run tauri:dev
     ou : npm run tauri:dev:usb   (sync + irecovery + tauri dev)
     ou build : npm run tauri:build   ou   npm run tauri:build:win-with-usb

Sans copie locale, PanicBase cherche encore les outils via :
  - PANICBASE_IDEVICE_DIR ou LIBIMOBILEDEVICE_HOME
  - %LOCALAPPDATA%\iDevicePanicLogAnalyzer\app-*\win-x64
  - %LOCALAPPDATA%\libimobiledevice
  - PATH

Test rapide : idevice_id -l  (avec l’iPhone branché et « Faire confiance » fait sur le téléphone)
