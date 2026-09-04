# Les clignotements du voile — la mesure avant, et la mesure après

**Écrit le 4 septembre 2026.** Ce document a deux moitiés : ce qui a été mesuré
sur le code d'avant le correctif, et la procédure pour re-mesurer après.

---

## 1. La base — mesurée par Thierry le 4 septembre 2026, sur `868ba0d`

Séance de banc, `CLICHE_BENCH=20`, transport A. Arbre propre.

| étape | médiane | p95 |
| --- | --- | --- |
| `capture` | 23,4 ms | 25,4 ms |
| `transport` | 1,4 ms | 1,6 ms |
| `shown` | 0,0 ms | 0,2 ms |
| `painted` | 91,3 ms | 94,3 ms |
| **TOTAL** | **115,3 ms** | **121,7 ms** |

**Ce sont les chiffres du rapport à 18 runs, et c'est délibéré.** Un second
rapport, à 20 échantillons, affichait 115,7 / 121,7 avec un p95 de `shown` à
14,0 ms. La lecture la plus probable — **à confirmer par Thierry, elle n'est pas
établie** — est que ce second rapport n'est pas comparable : `report_due` compte
TOUS les runs peints, y compris les pressions manuelles du raccourci, et les
trois observations de l'étape 3 ont été faites après le `bench: finished`. Le
rapport à 20 mélangerait donc 18 runs de banc et 2 captures à la main.

En attendant cette confirmation, la table ci-dessus est celle qui sert de
référence : elle ne contient que des runs produits par le banc.

### Deux faits relevés dans la même séance

**Les runs 1 et 2 n'ont pas accusé réception en 3 secondes.** Ce n'est pas la
chauffe : `BENCH_WARMUP` est un sommeil AVANT la boucle (`veil.rs`), et le
message vient de `BENCH_RUN_TIMEOUT`, le délai par run. Les deux premières
captures d'une application froide mettent donc plus de trois secondes à
répondre. Cartée à part — c'est ce fait qui rend le repli ci-dessous obligatoire.

**Les trois observations, sur le code d'avant :**

1. `Ctrl+Maj+2` puis Échap : un seul clignotement à l'ouverture (plus les « deux
   allumages » du matin), et un autre à l'Échap.
2. Clic sans glisser : le voile reste ouvert, la croix change, les poignées
   apparaissent. Ne fait plus « comme une photo ».
3. Sélection puis `Entrée` : fermeture après la copie, aucun retard perçu. Deux
   copies, 683×471 en 5,3 ms et 623×440 en 4,1 ms.

---

## 2. Ce que le correctif change

Avant, `perform_capture` faisait `window.show()` **puis** passait l'image à la
page. La fenêtre était donc visible pendant tout le décodage — **91,3 ms de
médiane** — en affichant ce que la page contenait de la capture précédente.
C'est la cause unique des clignotements.

Après, `perform_capture` ne montre plus rien : il passe l'image à une fenêtre
**cachée**. La page décode, appelle `veil_decoded`, et c'est cette commande qui
affiche la fenêtre. Le voile apparaît complet.

**Un repli à 250 ms** montre le voile quand même si l'accusé n'arrive pas. Sans
lui, un accusé manquant transformerait le raccourci en « rien ne se passe » — et
les runs 1 et 2 ci-dessus montrent que ce n'est pas théorique. Un run affiché par
le repli est **exclu des mesures** et le dit dans le terminal.

---

## 3. LE CONTRAT DE MESURE — lis-le avant de comparer quoi que ce soit

Cinq étapes au lieu de quatre. **Les deux bouts du TOTAL n'ont pas bougé** :
début de la saisie d'écran → la page a peint une fenêtre visible.

| étape | ce qu'elle mesure |
| --- | --- |
| `capture` | inchangé |
| `transport` | inchangé |
| `decoded` (neuve) | `eval` → fetch → décodage → accusé. **Fenêtre cachée.** |
| `shown` (redéfinie) | `show()` + `set_focus()`. **La fenêtre devient visible ici.** |
| `painted` (redéfinie) | une image d'animation après l'affichage, + accusé |

**Étape par étape, les deux rapports ne sont PAS comparables.** Seul le TOTAL
l'est.

### La prédiction, écrite avant de mesurer

- `decoded` ≈ l'ancien `painted` **moins** une image d'animation
- `shown` reste ≈ 0
- `painted` tombe à une image d'animation plus un aller-retour
- **TOTAL : en HAUSSE**, de l'ordre d'un aller-retour IPC supplémentaire

**Le correctif doit coûter quelques millisecondes, pas en faire gagner.** Ce
qu'il achète n'est pas dans le chiffre : c'est que la fenêtre ne devient jamais
visible avec du contenu périmé.

**Si le TOTAL BAISSE nettement, c'est un défaut, pas un gain** : une étape aura
cessé d'être comptée, et il faudra trouver laquelle avant d'y croire.

---

## 4. La procédure — environ 4 minutes

### Étape 1 — se placer

```powershell
cd F:\PROJECTS\Apps\cliche
git checkout main
git pull --ff-only
git status --short          # doit ne RIEN afficher
git log --oneline -1
```

Le dernier commit doit être **`fix(veil): show the veil only once the frame has
decoded`**.

### Étape 2 — les 20 mesures

```powershell
$env:CLICHE_BENCH = '20'; $env:CLICHE_TRANSPORT = 'bmp'; pnpm tauri dev
```

**Ton écran sera recouvert 20 fois, environ une fois par seconde, après 30 à
60 secondes de compilation.** Ne touche à rien pendant ces quarante secondes.

**Copie le bloc entier** du rapport, et **aussi toute ligne contenant
`FALLBACK`** : elle dirait qu'un run a été affiché par le repli.

⚠️ **Ne presse pas le raccourci à la main avant d'avoir copié le rapport.** Une
pression manuelle entre dans les mêmes compteurs et change le rapport suivant —
c'est très probablement ce qui s'est passé la dernière fois.

### Étape 3 — regarder, après avoir copié le bloc

**Ne ferme pas l'application.**

1. `Ctrl+Maj+2` puis **Échap**. Combien de clignotements, et où ?
2. `Ctrl+Maj+2`, sélection, `Entrée`. Le voile apparaît-il d'un coup, complet ?
3. Et la question qu'aucune mesure ne tranchera : **est-ce que ça te paraît plus
   lent ?** Avant, quelque chose apparaissait à ~25 ms, périmé. Maintenant, rien
   jusqu'à ~115 ms, puis le voile complet. Le chiffre peut être identique et la
   sensation différente. C'est toi qui décides.

### Pour finir

`Ctrl+C` dans le terminal, puis :

```powershell
Remove-Item Env:\CLICHE_BENCH, Env:\CLICHE_TRANSPORT
```

Sans ça, le prochain `pnpm tauri dev` **dans ce terminal** relancerait 20
clignotements.

---

## 5. Ce que je veux que tu me rendes

1. Le **bloc de mesure entier**, et toute ligne `FALLBACK`.
2. Les **trois réponses** de l'étape 3.
3. Si tu as pressé le raccourci à la main avant de copier le rapport — dis-le,
   ça change comment je lis les chiffres.

---

## 6. Ce qui reste NON VÉRIFIÉ, et doit l'être par cette séance

- **Le comportement d'un WebView2 caché n'a jamais été mesuré.** Le code d'avant
  portait un avertissement explicite contre cet ordre, parce que WebView2
  étrangle `requestAnimationFrame` dans une fenêtre non visible. Le correctif
  contourne l'objection — il n'utilise plus aucun `requestAnimationFrame` avant
  l'affichage, seulement `decode()` — mais **il ne l'a pas réfutée**. Si un
  WebView2 caché bloque aussi `decode()`, l'accusé n'arrivera jamais, et c'est le
  repli que tu verras, à chaque capture.
- **L'application n'a pas été lancée depuis le correctif.** Le nouvel ordre, le
  repli et l'unicité de l'affichage sont couverts par des tests sur les décisions
  pures ; le comportement réel, non.
