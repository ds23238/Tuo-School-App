# Importiert das TSA-Sync-Zertifikat, damit sich das MSIX installieren lässt.
# Einmalig als Administrator ausführen, im Ordner mit TSA-Sync.cer.
#Requires -RunAsAdministrator
Import-Certificate -FilePath "$PSScriptRoot\TSA-Sync.cer" -CertStoreLocation Cert:\LocalMachine\TrustedPeople
