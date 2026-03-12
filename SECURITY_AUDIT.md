# Rapport d'audit de sécurité — backend/src/main.rs

**Date :** 2026-03-11
**Périmètre :** `backend/src/main.rs` + `.env`
**Auditeur :** Analyse statique automatisée

---

## Résumé exécutif

| Criticité | Nombre |
|-----------|--------|
| CRITIQUE  | 4      |
| HAUTE     | 8      |
| MOYENNE   | 7      |
| FAIBLE    | 5      |
| **Total** | **24** |

---

## CRITIQUE

---

### [C-1] Secret JWT trivial en production
**Ligne :** `.env:3` / `main.rs:222`
**Criticité :** CRITIQUE

**Problème :**
```
JWT_SECRET=_ultra_securisee_123
```
Le secret JWT fait 20 caractères avec un préfixe prévisible (`_ultra_securisee_`). Un attaquant peut le bruteforcer via des outils comme `hashcat` en quelques secondes sur GPU. Toute personne connaissant ce secret peut forger des tokens JWT arbitraires avec n'importe quel `username` et `role`, y compris `super_admin`.

**Correction recommandée :**
Générer un secret cryptographiquement aléatoire d'au minimum 256 bits :
```bash
openssl rand -base64 64
```
Stocker le résultat dans `.env`. Ne jamais commiter `.env` en VCS.

---

### [C-2] Credentials MongoDB dans le fichier `.env` versionnable
**Ligne :** `.env:1`
**Criticité :** CRITIQUE

**Problème :**
```
DATABASE_URL=mongodb+srv://sisi:Saintjul54@clusterz.ox45wjs.mongodb.net/...
```
L'URI MongoDB contient le mot de passe en clair (`Saintjul54`) dans le fichier `.env`. Si ce fichier est committé dans le dépôt git (même une seule fois dans l'historique), les credentials sont compromis définitivement. L'URI est également exposée dans les logs si `DATABASE_URL` apparaît dans un message d'erreur.

**Correction recommandée :**
- Ajouter `.env` à `.gitignore` immédiatement.
- Vérifier l'historique git et supprimer tout commit contenant ces credentials.
- Changer le mot de passe MongoDB sans attendre.
- Envisager un gestionnaire de secrets (Vault, AWS Secrets Manager) en production.

---

### [C-3] JWT algorithm non contraint — vulnérabilité `alg:none`
**Ligne :** `134` (`Validation::default()`)
**Criticité :** CRITIQUE

**Problème :**
```rust
decode::<Claims>(
    token,
    &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
    &Validation::default(),  // ← DANGEREUX
)
```
`Validation::default()` dans la crate `jsonwebtoken` accepte par défaut l'algorithme `HS256` mais ne désactive pas explicitement les autres algorithmes. Selon la version de la crate, un token forgé avec `alg: "none"` ou `alg: "RS256"` combiné à une clé publique connue peut contourner la vérification. La liste des algorithmes autorisés doit être explicitement restreinte.

**Correction recommandée :**
```rust
let mut validation = Validation::new(Algorithm::HS256);
validation.leeway = 0;
decode::<Claims>(token, &key, &validation)
```
Appliquer la même correction à `ws_handler` (ligne 1015).

---

### [C-4] Injection NoSQL via le modérateur automatique
**Ligne :** `1122`
**Criticité :** CRITIQUE

**Problème :**
```rust
let _ = mod_db.collection::<shared::Message>("messages").update_one(
    doc! { "channel_id": &mod_channel, "content": &mod_content },
    doc! { "$set": { "deleted": true } },
    None
).await;
```
Le filtre de suppression utilise `content` comme critère de matching exact. Si deux messages différents partagent le même contenu (y compris des messages légitimes), **tous** seront marqués supprimés. Un attaquant peut provoquer la suppression en masse de messages légitimes en envoyant un message identique à un message existant contenant des mots-clés déclencheurs. De plus, la modération IA est non déterministe et peut générer des faux positifs.

**Correction recommandée :**
Filtrer par `_id` (ObjectId) du message inséré, pas par son contenu. Récupérer l'`inserted_id` au moment de l'insertion et le passer au modérateur.

---

## HAUTE

---

### [H-1] Token JWT transmis en query string (WebSocket)
**Ligne :** `1014-1015`
**Criticité :** HAUTE

**Problème :**
```rust
// WsQuery { token: String, channel: Option<String> }
// URL: /ws?token=<JWT>&channel=general
```
Le token JWT est transmis en paramètre d'URL pour la connexion WebSocket. Les tokens dans les URLs sont enregistrés dans :
- Les logs du serveur web / proxy
- L'historique du navigateur
- Les headers `Referer` envoyés aux iframes/ressources tierces
- Les access logs des CDN/reverse proxies

**Correction recommandée :**
Transmettre le token dans le premier message WebSocket (handshake applicatif) plutôt que dans l'URL. Côté client, envoyer `{"type":"auth","token":"..."}` immédiatement après connexion, et rejeter toute autre action tant que l'auth n'est pas validée.

---

### [H-2] Absence de headers de sécurité HTTP
**Ligne :** `326-350` (configuration du Router)
**Criticité :** HAUTE

**Problème :**
Aucun des headers de sécurité standard n'est configuré :
- `Strict-Transport-Security` (HSTS) — absent
- `X-Content-Type-Options: nosniff` — absent
- `X-Frame-Options: DENY` — absent
- `Content-Security-Policy` — absent
- `Referrer-Policy` — absent
- `Permissions-Policy` — absent

**Correction recommandée :**
Ajouter un middleware `tower_http::set_header::SetResponseHeaderLayer` ou utiliser `tower-http`'s `SecureHeaders` pour injecter ces headers sur toutes les réponses.

---

### [H-3] `GET /api/channels` non authentifiée — fuite de structure
**Ligne :** `615-626`
**Criticité :** HAUTE

**Problème :**
```rust
async fn channels_handler(State(state): State<AppState>) -> impl IntoResponse {
    // Pas de decode_token() — accessible sans JWT
```
La liste complète des canaux (noms, descriptions, métadonnées) est accessible sans authentification. Cela expose la structure interne du forum à tout visiteur anonyme et facilite la reconnaissance d'une cible.

**Correction recommandée :**
Ajouter `decode_token(&state, &headers)` en entrée du handler, ou appliquer un middleware d'authentification global.

---

### [H-4] Pas de validation du champ `role` dans `promote_handler`
**Ligne :** `837`
**Criticité :** HAUTE

**Problème :**
```rust
let _ = state.db.collection::<User>("users").update_one(
    doc! { "username": &req.username },
    doc! { "$set": { "role": &req.role } },
    None
).await;
```
La valeur `req.role` est insérée directement en base sans validation. Un admin peut fournir une valeur arbitraire comme `"root"`, `"god"`, `""`, ou une chaîne très longue. Seul le cas `"super_admin"` est bloqué (ligne 832), mais les autres valeurs invalides passent.

**Correction recommandée :**
Valider que `req.role` est une valeur parmi `["user", "admin"]` (le rôle `super_admin` est déjà protégé). Retourner `400 Bad Request` sinon.

---

### [H-5] Rate limit WebSocket insuffisant et partagé avec le chat
**Ligne :** `162-180` / `1063`
**Criticité :** HAUTE

**Problème :**
```rust
is_valid = times.len() < 10;  // 10 messages/60s
```
La fonction `check_rate_limit` est utilisée à la fois pour les messages de chat (WS) et potentiellement pour d'autres opérations. La clé est le `username` (depuis le JWT), ce qui est correct, mais :
1. Le seuil de 10 messages/60s est trop permissif pour du flood coordonné
2. Aucun rate limit n'existe sur la connexion WebSocket elle-même (reconnexions en boucle)
3. Aucun rate limit sur `POST /api/register` — permet la création de comptes en masse

**Correction recommandée :**
- Limiter les inscriptions par IP (5/heure).
- Limiter les reconnexions WS par IP.
- Réduire le seuil de messages à 5/10s (burst) + 30/min (soutenu).

---

### [H-6] `delete_account_handler` sans confirmation ni protection
**Ligne :** `797-804`
**Criticité :** HAUTE

**Problème :**
```rust
let _ = state.db.collection::<User>("users").delete_one(
    doc! { "username": &claims.username }, None
).await;
```
La suppression de compte est définitive et n'est pas soft-delete. Les messages de l'utilisateur restent en base référençant un `author_name` qui n'existe plus. De plus, il n'y a aucune protection contre le CSRF (un site tiers peut déclencher cet appel si les cookies sont configurés), bien que l'authentification par Bearer token rende cela peu probable depuis un navigateur standard.

**Correction recommandée :**
Implémenter un soft-delete (`deleted: true` sur le user), anonymiser les messages associés, et requérir la confirmation du mot de passe actuel.

---

### [H-7] Contenu des messages IA injecté sans sanitization
**Lignes :** `428`, `990`, `587`
**Criticité :** HAUTE

**Problème :**
Les réponses de l'API Qwen/Colab sont insérées directement en base de données et diffusées aux clients sans aucune sanitization :
```rust
j["response"].as_str().unwrap_or(&title).trim().to_string()
// → inséré tel quel dans le contenu du message
```
Si le service IA est compromis ou renvoie du contenu malveillant (XSS, injection), ce contenu sera stocké et servi aux utilisateurs. La fonction `sanitize_message` n'est pas appliquée aux réponses IA.

**Correction recommandée :**
Appliquer `sanitize_message()` sur toutes les réponses IA avant stockage. Limiter la longueur des réponses IA.

---

### [H-8] COLAB_AI_URL sans vérification de certificat TLS ni authentification
**Ligne :** `357-358`, `981-986`
**Criticité :** HAUTE

**Problème :**
L'URL Colab est un endpoint ngrok (`ngrok-free.dev`) sans authentification côté serveur. Le client reqwest ne valide pas spécifiquement le certificat, et ngrok peut changer d'URL à tout moment. Un attaquant qui contrôle l'URL (via MITM ou si le tunnel ngrok est réassigné) peut injecter du contenu arbitraire dans les messages du forum.

**Correction recommandée :**
- Ajouter une clé API secrète dans les requêtes vers le service IA (header `X-Api-Key`).
- Vérifier que l'URL est bien HTTPS en production.
- Ajouter un champ `required: bool` pour ne pas insérer de message si l'IA échoue.

---

## MOYENNE

---

### [M-1] JWT sans champ `nbf` (not before) ni rotation possible
**Ligne :** `1291-1296`
**Criticité :** MOYENNE

**Problème :**
Les tokens JWT ont une durée de vie de 24h sans mécanisme de révocation. Un token volé reste valide jusqu'à son expiration. Aucun champ `nbf` n'est émis. Aucune liste noire (blacklist) ou mécanisme de refresh token n'existe.

**Correction recommandée :**
Implémenter une blocklist en mémoire (DashMap) ou en Redis pour les tokens invalidés (logout, ban). Réduire la durée à 1-2h avec refresh token. Ajouter le champ `nbf` = `iat`.

---

### [M-2] `regex::Regex::new()` compilé à chaque requête
**Ligne :** `1198`, `1203`
**Criticité :** MOYENNE

**Problème :**
```rust
let username_re = regex::Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap();
let email_re = regex::Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
```
Les regex sont recompilées à chaque appel de `register_handler`. Sous charge élevée, cela représente un coût CPU inutile et un risque de déni de service partiel (ReDoS si la regex devient complexe).

**Correction recommandée :**
Utiliser `once_cell::sync::Lazy<Regex>` ou `std::sync::LazyLock<Regex>` pour initialiser les regex une seule fois au démarrage.

---

### [M-3] Taille du body JSON non limitée
**Ligne :** `326-350` (Router)
**Criticité :** MOYENNE

**Problème :**
Axum accepte des payloads JSON de taille illimitée par défaut. Un attaquant peut envoyer un body de plusieurs gigaoctets pour épuiser la mémoire du serveur, provoquant un déni de service. Le seul check de taille actuel concerne les avatars (ligne 792 : 500 000 octets).

**Correction recommandée :**
Ajouter un middleware de limitation globale :
```rust
use axum::extract::DefaultBodyLimit;
.layer(DefaultBodyLimit::max(1_000_000)) // 1 MB
```

---

### [M-4] Absence de pagination sur `GET /api/messages`
**Ligne :** `649`
**Criticité :** MOYENNE

**Problème :**
```rust
let options = FindOptions::builder().sort(doc! { "created_at": 1 }).build();
// Aucun .limit()
```
Tous les messages d'un salon depuis le début des temps sont retournés en une seule requête. Pour un salon actif (ex: `infos` avec des flux RSS), cela peut représenter des milliers de documents, saturant la mémoire et la bande passante.

**Correction recommandée :**
Ajouter `.limit(200)` et implémenter une pagination par cursor (`before_id` ou `offset`).

---

### [M-5] `me_handler` retourne l'email de l'utilisateur
**Ligne :** `746`
**Criticité :** MOYENNE

**Problème :**
```rust
"email": user.email,
```
L'endpoint `/api/users/me` retourne l'adresse email de l'utilisateur dans la réponse JSON. Si ce endpoint est accessible à d'autres utilisateurs (il ne l'est pas actuellement), cela constituerait une fuite de données personnelles. Même sans cela, l'email est visible dans les DevTools du navigateur et dans tout proxy intermédiaire.

**Correction recommandée :**
Évaluer si l'email est réellement nécessaire côté frontend. Si oui, s'assurer que cet endpoint n'est jamais accessible par d'autres utilisateurs que le propriétaire du token.

---

### [M-6] Le filtre de modération est trop facile à contourner
**Ligne :** `1089-1091`
**Criticité :** MOYENNE

**Problème :**
```rust
let flagged = content.len() > 50 && bad_words.iter().any(|w| lower.contains(w));
```
La liste de mots déclencheurs est statique et triviale à contourner (espaces insérés, l33tspeak, variantes orthographiques). De plus, la condition `content.len() > 50` permet d'envoyer des insultes courtes sans déclenchement.

**Correction recommandée :**
Supprimer la condition de longueur. Compléter la liste ou déléguer entièrement la décision à l'IA. Traiter les variantes courantes (l33tspeak, espaces) avant la comparaison.

---

### [M-7] `channel_id` non validé dans `messages_handler`
**Ligne :** `636`
**Criticité :** MOYENNE

**Problème :**
```rust
let filter = doc! {
    "channel_id": &params.channel,
    ...
};
```
La valeur de `params.channel` (paramètre de query string) est insérée directement dans le filtre MongoDB sans validation de format ni liste blanche. Bien que le driver MongoDB la traite comme une chaîne de caractères (pas une injection BSON), une valeur arbitrairement longue ou contenant des caractères spéciaux peut causer des comportements inattendus.

**Correction recommandée :**
Valider que `channel` correspond à un identifiant connu (vérification en DB ou regex `^[a-z0-9_-]{1,50}$`).

---

## FAIBLE

---

### [F-1] Logs d'informations trop verbeux en production
**Ligne :** `1301`
**Criticité :** FAIBLE

**Problème :**
```rust
info!("Connexion: {}", response.username);
```
Les usernames de connexion sont logués. Sans rotation et protection des logs, cela peut faciliter le profilage des utilisateurs actifs.

**Correction recommandée :**
En production, limiter les logs de connexion ou les anonymiser. Utiliser un niveau `debug` plutôt qu'`info`.

---

### [F-2] La liste noire d'URLs suspectes est incomplète
**Ligne :** `192-195`
**Criticité :** FAIBLE

**Problème :**
```rust
let suspicious = ["bit.ly", "tinyurl", ".exe", ".zip", "discord.gg/"];
```
La liste est trop courte (pas de `t.co`, `is.gd`, `cutt.ly`, etc.) et les extensions malveillantes sont incomplètes (pas de `.bat`, `.ps1`, `.msi`, `.dmg`). Un attaquant peut contourner facilement via d'autres raccourcisseurs ou extensions.

**Correction recommandée :**
Utiliser une approche positive (liste blanche de domaines autorisés) plutôt que négative, ou intégrer un service de réputation d'URL.

---

### [F-3] Absence de Content-Type strict sur les routes POST
**Ligne :** `326-350`
**Criticité :** FAIBLE

**Problème :**
Axum accepte par défaut des requêtes POST sans vérifier strictement le `Content-Type: application/json`. Des clients malformés ou du contenu CSRF peuvent envoyer des données sous d'autres formats.

**Correction recommandée :**
Le `JsonExtractor` d'Axum rejette déjà les Content-Type invalides en retournant 415, mais documenter explicitement ce comportement et le tester.

---

### [F-4] `unwrap_or_default()` silencieux sur des ObjectId invalides dans `react_to_message_handler`
**Ligne :** `812-815`
**Criticité :** FAIBLE

**Problème :**
```rust
let oid = match ObjectId::parse_str(&req.message_id) {
    Ok(i) => i,
    Err(_) => return (StatusCode::BAD_REQUEST, "ID invalide").into_response(),
};
```
Ce point est correct. Cependant, `req.message_id` n'est pas validé pour sa longueur maximum avant le parse. Une chaîne excessivement longue sera parsée (et échouera) mais après allocation.

**Correction recommandée :**
Ajouter une validation préalable : `if req.message_id.len() != 24 { return BAD_REQUEST }`.

---

### [F-5] Informations de version du serveur potentiellement exposées
**Ligne :** `326-350`
**Criticité :** FAIBLE

**Problème :**
Axum ajoute par défaut un header `Server: axum` sur certaines réponses d'erreur (notamment 404, 405). Cela révèle le framework utilisé et facilite le ciblage de vulnérabilités spécifiques à Axum.

**Correction recommandée :**
Ajouter un middleware qui supprime ou remplace le header `Server` :
```rust
.layer(SetResponseHeaderLayer::overriding(
    header::SERVER,
    HeaderValue::from_static(""),
))
```

---

## Récapitulatif des routes et leur niveau de protection

| Route | Méthode | JWT requis | Rôle minimum | Commentaire |
|-------|---------|-----------|-------------|-------------|
| `/api/health` | GET | ❌ | — | Normal |
| `/api/register` | POST | ❌ | — | Pas de rate limit par IP |
| `/api/login` | POST | ❌ | — | Rate limit OK (5/min) |
| `/api/channels` | GET | ❌ | — | **[H-3] Non protégé** |
| `/api/channels` | POST | ✅ | Admin | OK |
| `/api/channels/discover` | GET | ✅ | User | OK |
| `/api/channels/:id` | DELETE | ✅ | Admin | OK |
| `/api/channels/:id/summary` | POST | ✅ | SuperAdmin | OK |
| `/api/channels/:id/archive` | GET | ✅ | SuperAdmin | OK |
| `/api/messages` | GET | ✅ | User | Pas de pagination |
| `/api/messages/react` | POST | ✅ | User | OK |
| `/api/messages/:id` | DELETE | ✅ | Admin | OK |
| `/api/messages/:id/pin` | POST | ✅ | Admin | OK |
| `/api/users` | GET | ✅ | User | OK |
| `/api/users/me` | GET | ✅ | User | Retourne email |
| `/api/users/promote` | POST | ✅ | Admin | Validation rôle manquante |
| `/api/users/ban` | POST | ✅ | Admin | OK |
| `/api/users/channels/subscribe` | POST | ✅ | User | OK |
| `/api/users/avatar` | POST | ✅ | User | OK |
| `/api/users/delete` | DELETE | ✅ | User | Suppression définitive |
| `/api/direct_messages` | GET | ✅ | User | OK |
| `/api/direct_messages/read` | POST | ✅ | User | OK |
| `/ws` | GET | ✅ (query) | User | **[H-1] Token en URL** |

---

## Actions prioritaires (ordre d'urgence)

1. **[C-1]** Remplacer `JWT_SECRET` par une clé aléatoire ≥ 256 bits
2. **[C-2]** Changer le mot de passe MongoDB, ajouter `.env` au `.gitignore`
3. **[C-3]** Restreindre explicitement l'algorithme JWT à `HS256`
4. **[H-2]** Ajouter les headers de sécurité HTTP (`HSTS`, `X-Frame-Options`, `CSP`, etc.)
5. **[H-1]** Migrer l'authentification WebSocket hors de la query string
6. **[H-3]** Protéger `GET /api/channels` par JWT
7. **[C-4]** Corriger la suppression par contenu dans le modérateur automatique
8. **[M-3]** Limiter la taille des payloads JSON entrants
