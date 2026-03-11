#!/bin/bash
# Script de sauvegarde MongoDB quotidien

# Configuration
DB_NAME="rust_chat_forum"
BACKUP_DIR="./backups"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/backup_$DATE"

# Créer le dossier de backup s'il n'existe pas
mkdir -p $BACKUP_DIR

# Faire le backup
echo "📦 Sauvegarde MongoDB en cours..."
mongodump --db $DB_NAME --out $BACKUP_FILE

# Supprimer les backups de plus de 7 jours
find $BACKUP_DIR -type d -mtime +7 -exec rm -rf {} \;

echo "✅ Backup terminé : $BACKUP_FILE"
echo "🗂️  Backups de plus de 7 jours supprimés"
