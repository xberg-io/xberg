# Plan de Parité PII Browser ↔ MCP

**Branche :** `feature/wasm-runtime-sqlite-store`
**Date :** 2026-07-10
**Contexte :** L'utilisateur ingère des documents dans le browser (via WASM runtime), et le MCP server les consomme en toute sécurité. Le pipeline PII doit garantir la même protection aux deux bouts.

Tout point ci-dessous a été vérifié par lecture de code (citations `fichier:ligne`).

---

## État des Lieux Vérifié

| # | Point | Statut | Preuve |
|---|-------|--------|--------|
| V1 | `resolve_ingest_ner` retourne `None` quand pas de JS bridge, même si Candle est chargé | ✅ | `bridge/ner.rs:169-180` |
| V2 | JS bridge écrase toujours Candle dans `ingest()` | ✅ | `engine.rs:187-188` |
| V3 | `ingest_folder` passe texte TS-rédacté à `engine.ingest()` | ✅ | `ingest.ts:250,274` |
| V4 | `engine.ingest()` re-redacte (NER inference gâchée) | ✅ | `pipeline.rs:666` |
| V5 | `scan_text()` ne matche pas `[EMAIL_1]` (regex requiert `@`) | ✅ | `email.rs:12` |
| V6 | `detect_pii` MCP n'utilise pas le NER | ✅ | `engine.rs:356-361` |
| V7 | `redact_document` MCP n'utilise pas le NER | ✅ | `engine.rs:371-433` |
| V8 | Browser envoie texte brut à `engine.ingest()` | ✅ | `engine.rs:158` |
| V9 | `NerBackend` est object-safe (déjà `Box<dyn>`) | ✅ | `backend.rs:11-19` |
| V10 | `engine.rs` n'est pas câblé dans `lib.rs` (`mod engine;` absent) | ✅ | `lib.rs` grep |
| V11 | `detect_pii`/`redact` TS sont déjà `await`-ed | ✅ | `pii.ts:73,118` |

---

## Étape 1 — `resolve_ingest_ner` consulte aussi Candle

**Problème :** `resolve_ingest_ner` ignore le Candle thread-local. La logique Candle est en dehors de la fonction, dans `engine.rs`, ce qui rend la fonction non-réutilisable et fragile.

**Fichier :** `crates/xberg-wasm/src/bridge/ner.rs:169-180`

```rust
// APRÈS
pub(crate) fn resolve_ingest_ner(
    injected: Option<&js_sys::Object>,
    timeout_ms: u32,
) -> Option<Box<dyn NerBackend>> {
    if let Some(obj) = injected {
        return Some(Box::new(JsNerBridge::new(obj.clone(), timeout_ms)));
    }
    if let Some(candle) = get_candle_ner() {
        struct CandleBox(std::rc::Rc<xberg::text::ner::candle::CandleBackend>);
        #[async_trait(?Send)]
        impl xberg::text::ner::NerBackend for CandleBox {
            async fn detect(&self, text: &str, categories: &[xberg::types::entity::EntityCategory])
                -> xberg::Result<Vec<xberg::types::entity::Entity>> {
                self.0.detect(text, categories).await
            }
        }
        return Some(Box::new(CandleBox(candle)));
    }
    None
}
```

**Fichier :** `crates/xberg-wasm/src/engine.rs:171-195` — supprimer le `CandleBox` dupliqué et simplifier :

```rust
let ner_backend = crate::bridge::ner::resolve_ingest_ner(self.ner.as_ref(), self.bridge_timeout_ms)
    .ok_or_else(|| JsValue::from_str(
        "PII detection unavailable: inject a ner bridge or call initCandleNer",
    ))?;
```

**Tests :**
- `resolve_ingest_ner(None, t)` avec Candle chargé → `Some`
- `resolve_ingest_ner(Some(obj), t)` avec Candle chargé → `Some(JsNerBridge)` (JS gagne)

---

## Étape 2 — Éliminer la double rédaction dans `ingest_folder`

**Problème :** `ingest_folder` rédacte le texte en TS (lignes 244-250), puis passe `redactedText` à `engine.ingest()` (ligne 274) qui re-redacte via `redact_request()` (NER inference gaspillée, `pipeline.rs:666`). La `RehydrationMap` retournée par `engine.ingest()` est jetée (seul `docId` est lu, `ingest.ts:300`).

**Recommandation :** Passer `rawText` (déjà disponible ligne 243) à `engine.ingest()`. Les fichiers `_REDACTED` et `.map` restent produits par la rédaction TS (pour les sorties disque), mais le stockage RAG est rédigé une seule fois par le Rust.

**Fichier :** `mcp-server/src/tools/ingest.ts:273-274`

```typescript
// APRÈS
const ingestDoc = toIngestRequest({
    full_text: rawText,  // ← texte brut : engine.ingest() rédacte une seule fois
    title: filename,
    mime: doc.mimeType,
    source_uri: filePath,
    external_id: `${collection}-${baseName}`,
    ...
});
```

**Tests :**
- Ingérer un doc avec email → vector store contient `[EMAIL_1]`
- Fichier `_REDACTED` contient `[EMAIL_1]` (rédaction TS)
- Fichier `.map` contient `[EMAIL_1] → alice@example.com` (rédaction TS)

---

## Étape 3 — Ajouter le NER aux outils `detect_pii`/`redact_document`

**Problème :** `detect_pii` et `redact_document` (MCP) n'utilisent que `scan_text()` (regex). Les noms/personnes/Organizations/Locations ne sont pas détectés. Un agent IA qui demande "détecte les noms" reçoit un résultat vide.

**Fichier :** `crates/xberg-wasm/src/engine.rs`

Convertir `detect_pii` (ligne 356) et `redact` (ligne 371) en `pub async fn` et router via `resolve_ner_with_timeout(self.ner.clone(), …, self.bridge_timeout_ms).await` (comme `ner()` ligne 298).

**Préconditions :**
- (a) Câbler `engine.rs` dans `lib.rs` (`mod engine;`) — nécessaire car `engine.rs` est actuellement orphelin (V10).
- (b) Ajouter `await` au call nu `pii.ts:128` (`engine.detect_pii(text)` → `await engine.detect_pii(text)`).

**Changement `detect_pii` :**
```rust
#[allow(clippy::missing_errors_doc)]
pub async fn detect_pii(&self, text: String, categories: Option<Vec<String>>) -> Result<JsValue, JsValue> {
    let cats: Vec<xberg::types::redaction::PiiCategory> =
        categories.unwrap_or_default().into_iter().map(Into::into).collect();
    let mut matches = xberg::text::redaction::patterns::scan_text(&text, &cats);
    // NER merge (si bridge injecté ou Candle chargé)
    let ner_cats = vec![
        xberg::types::entity::EntityCategory::Person,
        xberg::types::entity::EntityCategory::Organization,
        xberg::types::entity::EntityCategory::Location,
    ];
    if let Ok(entities) = resolve_ner_with_timeout(self.ner.clone(), &text, &ner_cats, self.bridge_timeout_ms).await {
        for e in entities {
            let cat = match e.category {
                xberg::types::entity::EntityCategory::Person => xberg::types::redaction::PiiCategory::Person,
                xberg::types::entity::EntityCategory::Organization => xberg::types::redaction::PiiCategory::Organization,
                xberg::types::entity::EntityCategory::Location => xberg::types::redaction::PiiCategory::Location,
                _ => continue,
            };
            matches.push(xberg::text::redaction::patterns::PatternMatch {
                start: e.start as usize, end: e.end as usize, category: cat, text: e.text,
            });
        }
    }
    let matches = xberg::text::redaction::engine::dedupe_overlaps(matches);
    serde_wasm_bindgen::to_value(&matches).map_err(|e| JsValue::from_str(&e.to_string()))
}
```

**Changement `redact` :** identique mais appliquer `apply_strategy` + `apply_replacements_reverse` sur le texte fusionné.

**Tests :**
- `detect_pii("Contact Alice at alice@example.com")` → trouve `[PERSON]` + `[EMAIL]`
- `redact_document("Alice works at Acme")` → `[PERSON_1] works at [ORGANIZATION_1]`

---

## Séquence d'implémentation

1. **Étape 1** (Rust) — `resolve_ingest_ner` + `engine.ingest` simplification
2. **Étape 3a** (Rust) — câbler `engine.rs` dans `lib.rs`
3. **Étape 3b** (Rust) — `detect_pii`/`redact` async + NER
4. **Étape 2** (TS) — `ingest_folder` passe `rawText`
5. **Étape 3c** (TS) — `await` sur `pii.ts:128`
6. Build + tests

## Vérification

- `cargo check -p xberg-wasm --target wasm32-unknown-unknown` (job `wasm-check` CI)
- `cargo test -p xberg-rag --features pipeline-redaction` (tests redaction)
- `cd mcp-server && npm run build` (tsc)
- `cd packages/xberg-wasm-runtime && npx vitest run` (browser tests)
