# Design — Interface web d'ingestion (WASM) consommée par le MCP

**Date :** 2026-07-10
**Branche :** `feature/wasm-runtime-sqlite-store`
**Statut :** Design validé — architecture d'ensemble. Chaque lot aura son propre cycle spec → plan.

## Objectif

Offrir à l'utilisateur une interface web pour **créer des dossiers** et **uploader des
documents** qui sont extraits, OCRisés, passés au NER et au PII **dans le navigateur (WASM)**,
puis **consommés par le MCP** de Claude Desktop via ses tools existants
(`query_corpus`, `get_document`, `rehydrate_tokens`, …).

Principe directeur : **le WASM est le moteur d'ingestion**, le **MCP est le consommateur** et
le **propriétaire du store disque**. Le PII est rédigé côté navigateur **avant** tout envoi ;
le disque du MCP ne contient jamais de PII en clair.

## Décisions verrouillées

| Décision | Choix | Raison |
|----------|-------|--------|
| Où tourne l'ingestion | Navigateur (WASM runtime) | Réutilise `xberg-wasm-runtime` ; PII rédigé côté client |
| Propriétaire du store lu par le MCP | MCP (SQLite disque, `store-node.ts`) | Source de vérité unique pour Claude Desktop |
| Pont navigateur → MCP | Payloads structurés (upsert) via HTTP localhost | Incrémental, réutilise `RagStore::upsert` idempotent |
| Sync | Automatique après chaque document | Pas de « publier » manuel |
| Réplication SQLite brut | Rejetée | Deux écrivains sur un même fichier = fragile/corruption |
| Rehydration map | Poussée **chiffrée** au MCP | Permet `rehydrate_tokens` côté Claude Desktop |
| Navigateurs cibles | Chrome / Edge | Requis pour OPFS + isolation cross-origin |
| Auth | Token localhost partagé | Local mono-utilisateur pour le moment |
| Stack UI | Next.js (export statique), Radix, shadcn/ui, TanStack (Query + Table), extend-hq/ui | Composants documentaires prêts à l'emploi |

## Architecture

Un seul process à lancer : **le MCP server**, qui gagne deux responsabilités en plus de ses
tools actuels :

1. **Sert l'UI web** (assets statiques + `.wasm`) sur localhost.
2. **Possède le SQLite disque** (`store-node.ts`) — source de vérité interrogée par Claude Desktop.

```
┌─ Chrome/Edge ──────────────┐        ┌─ MCP server (Node, 1 process) ─┐
│ UI (dossiers, upload)      │        │  HTTP: /ui  /ingest  /map      │
│ WASM runtime :             │ POST   │  ┌──────────────────────────┐  │
│  extract→OCR→NER→PII→embed │──────► │  │ store-node.ts (SQLite)   │◄─┼── Claude Desktop
│  (OPFS = cache local)      │ upsert │  │  + rehydration maps chiff.│  │   (tools MCP)
└────────────────────────────┘        │  └──────────────────────────┘  │
                                       └────────────────────────────────┘
```

Le navigateur fait tout le calcul via le WASM runtime et ne persiste rien d'autoritaire
(l'OPFS sert de cache/preview local). Seuls le **texte rédigé + vecteurs + métadonnées** et la
**map chiffrée** quittent la page.

## Composants & interfaces

### Côté MCP (Node)

- **`transports/http.ts` (étendu)** — 3 routes ajoutées au serveur existant :
  - `GET /ui/*` → sert les assets statiques (bundle UI + `.wasm`), avec headers
    `COOP: same-origin` + `COEP: require-corp` (requis pour OPFS/SharedArrayBuffer).
  - `POST /ingest` → reçoit un `IngestPayload`, applique `store.upsert()` (idempotent).
  - `POST /map` → reçoit la rehydration map **chiffrée** (`XPII\x01` + nonce + ciphertext),
    l'écrit **hors du store** (fichier `.map` par document, dossier dédié).
  - `POST /admin` (ou routes dédiées) → expose `drop_collection` / `delete_documents` pour la
    re-ingestion et la suppression depuis l'UI.
- **`ui-server.ts` (nouveau)** — module isolé montant les routes ci-dessus pour garder
  `http.ts` mince. Auth : token localhost partagé, imprimé au démarrage du MCP, lu par l'UI.

### Côté navigateur (nouveau package `packages/xberg-web-ui/`)

- **UI Next.js (export statique)** — écrans dossiers/upload/document/sync.
- **`ingest-controller.ts`** — orchestre le WASM runtime existant
  (`factory.ts`, `ocr.ts`, `ner.ts`, `pii.ts`) puis appelle `sync-client.ts`.
- **`sync-client.ts`** — POST auto vers `/ingest` et `/map` après chaque document
  (retry + backoff). C'est le pont OPFS → disque.

### Réutilisé tel quel

Tout `xberg-wasm-runtime` (moteur), le trait `RagStore` / `upsert` de `store-node.ts`, la
couche `redaction/`.

## Flux de données

1. **Upload** (extend `FileUpload`) → fichier en mémoire navigateur, jamais envoyé brut.
2. **Pipeline WASM** : `extract` → `OCR` si scan/image (tesseract-wasm) → `NER` (Candle) →
   `detect_pii` + `redact` → `chunk` → `embed`. Progression streamée à l'UI (TanStack Query).
3. **Rédaction PII côté navigateur** : produit `redactedText`, `detections` (catégories +
   offsets, **jamais de valeurs**), et la `rehydrationMap` chiffrée (scrypt + AES-256-GCM).
4. **Auto-sync** (`sync-client.ts`) : `POST /ingest` (payload) puis `POST /map` (blob chiffré).
5. **MCP** : `store.upsert()` + écriture du `.map` hors store. **Claude Desktop** lit ensuite
   via `query_corpus`, `get_document`, `rehydrate_tokens`.

### Contrat `IngestPayload` (navigateur → MCP)

```jsonc
{
  "collectionId": "dossier-client-x",      // = "dossier" dans l'UI
  "sourceId": "contrat-2026.pdf",           // idempotence : (collectionId, sourceId, chunkIndex)
  "title": "Contrat 2026",
  "mime": "application/pdf",
  "redactedFullText": "…[EMAIL_1]…",        // JAMAIS de PII en clair
  "chunks": [{ "index": 0, "text": "…", "embedding": [/* f32 */] }],
  "metadata": { "pageCount": 12, "ocrUsed": true, "ingestedAt": "…" },
  "piiSummary": { "email": 3, "person": 2 }  // compteurs seulement
}
```

Le `.map` chiffré voyage **séparément** (`/map`). La clé n'est jamais persistée côté MCP ; le
passphrase est fourni à l'appel `rehydrate_tokens` (règle `pii-pipeline`).

### Idempotence & re-ingestion

- `upsert` idempotent sur `(collectionId, sourceId, chunkIndex)` : ré-uploader un fichier
  **remplace** ses chunks, pas de doublons.
- Re-ingestion / suppression de dossier depuis l'UI → routes HTTP mappées sur
  `drop_collection` / `delete_documents`.

## Contraintes techniques

- **Isolation cross-origin obligatoire** : OPFS + sqlite-vec (et threads WASM) exigent
  `COOP: same-origin` + `COEP: require-corp` sur le serveur statique du MCP.
- **UI en export statique** (`next build` avec `output: 'export'`) : pas de SSR/API routes — le
  backend est le MCP. Le WASM runtime tourne côté client (`'use client'`), dans un worker.
- **Navigateurs** : Chrome / Edge (File System Access + OPFS + isolation).

## Stack UI

- **Next.js** `output: 'export'` (SSG pur).
- **shadcn/ui + Radix**, **TanStack Query** (ingestion/statut) + **TanStack Table** (listes), Tailwind.
- **extend-hq/ui** (`npx shadcn add @extend/...`), mappé aux écrans :
  - `FileUpload` → upload dans un dossier
  - `FileSystem` (finder) → navigation dossiers/documents
  - `PDF/DOCX/XLSX Viewer` → visualisation d'un document
  - `LayoutBlocks` → rendu OCR + scores de confiance
  - `BoundingBoxCitations` → revue des détections PII sur le document
  - `FileThumbnail`, `DocumentSplitting` → vignettes / découpe multi-docs

### Écrans (V1)

1. Liste des dossiers.
2. Dossier → documents + upload.
3. Document → viewer + détections PII + statut.
4. Barre de sync globale.

## Tests

- **UI** : Vitest + Testing Library (composants), Playwright (parcours upload → sync ;
  réutilise le harnais Playwright existant du runtime).
- **Routes MCP** : intégration sur `/ingest`, `/map`, idempotence `upsert`, écriture `.map`
  hors store.
- **Runtime** : suites existantes inchangées.
- **E2E de bout en bout** : upload d'un PDF avec PII → le store MCP contient `[EMAIL_1]` et
  `rehydrate_tokens` restitue la valeur via la map chiffrée.

## Découpage en lots (chacun son spec → plan)

- **Lot 0** — fiabiliser le pipeline PII (plan existant
  `docs/superpowers/plans/2026-07-10-pii-browser-mcp-parity.md`). **Prérequis.**
- **Lot 1** — routes MCP `/ingest` `/map` + service statique + COOP/COEP + auth token.
- **Lot 2** — package `xberg-web-ui` (Next/shadcn/extend) + `sync-client` + pipeline d'upload.
- **Lot 3** — visualisation avancée (LayoutBlocks / BoundingBox / viewers) + re-ingestion /
  suppression.

## Hors périmètre (V1)

- Multi-utilisateur / vrai login (token localhost suffit pour l'instant).
- Firefox / Safari.
- Service de sync distant / hébergé multi-clients.
- Réplication du fichier SQLite brut.
