# Cliché — plan d'implémentation

> Document de travail, en français (c'est notre langue d'échange).
> **Le code, les commentaires, les commits et le README seront en anglais**, selon la
> convention du profil. Ce fichier est la seule exception assumée.
>
> Établi le **2 septembre 2026**. Toute date ci-dessous est absolue.

## Objectif

Un utilitaire Windows local qui capture (zone / fenêtre / écran / page web défilante),
annote, copie, enregistre et retrouve — en moins d'une seconde du raccourci au
presse-papier. Aucune donnée ne quitte la machine.

## Décisions arrêtées le 2 septembre 2026

| Sujet | Décision | Pourquoi |
| --- | --- | --- |
| Pile | **Tauri 2.11.5** + Rust + React/TS | Voir `docs/STACK.md` |
| Réseau | **Aucun**. Pas de compte, pas de lien de partage, pas de télémétrie | Demande explicite |
| Défilement | **Pages web d'abord** (lot 7). N'importe quelle fenêtre : hors périmètre v1 | Le recollage générique est fragile ; on ne paie pas ce prix avant d'avoir le reste |
| Plateformes | Code portable (trait `Capturer`), **livraison Windows seule** | La couche capture est irréductiblement Windows |
| Aide intégrée | **Lot 2**, juste après le squelette — pas à la fin | Demande explicite |

## État de la machine — mesuré le 2 septembre 2026

```
rustc 1.94.1 / cargo 1.94.1 / node v24.15.0 / pnpm 11.7.0
WebView2 152.0.4191.53 (présent)
Écrans : 1 seul — \\.\DISPLAY1, 1920x1080 @ (0,0), mise à l'échelle 100 %
Machine : MS-7D98, PCSystemType=1 (tour fixe)
```

**Conséquence directe, à ne pas oublier** : le code multi-écran et DPI mixtes sera
écrit correctement par construction, mais **ne pourra pas être vérifié ici**. Il est
marqué `NON VÉRIFIÉ` partout où il apparaît. Un second écran, même une TV en HDMI,
lèverait cette dette en une session.

---

## Lot 0 — Squelette Tauri qui démarre

**Fichiers** : `package.json` · `pnpm-workspace.yaml` · `index.html` · `vite.config.ts` ·
`tsconfig.json` · `src/main.tsx` · `src/App.tsx` · `src-tauri/Cargo.toml` ·
`src-tauri/build.rs` · `src-tauri/tauri.conf.json` · `src-tauri/src/main.rs` ·
`src-tauri/src/lib.rs` · `.gitignore`

**Fini quand** :

- `pnpm tauri dev` ouvre une fenêtre portant le titre « Cliché » — constaté à l'écran.
- `cargo build --manifest-path src-tauri/Cargo.toml` sort en 0.
- `pnpm typecheck` sort en 0.
- L'application est déclarée **per-monitor DPI aware v2** dans son manifeste, et
  journalise au démarrage : nombre d'écrans, résolution physique, facteur d'échelle.
  Attendu ici : `1 écran, 1920x1080, scale 1.0`.

**Risque** : désaccord entre `tauri-cli` 2.11.4 et le crate `tauri` 2.11.5, ou
WebView2 non trouvé. *On le verrait à* : `pnpm tauri dev` qui échoue au premier lancement.

---

## Lot 1 — Le squelette qui MESURE (lot critique, tout en dépend)

C'est le lot qui décide si la pile tient. Rien d'autre ne se construit avant qu'il soit vert.

**Fichiers** : `src-tauri/src/capture/mod.rs` (trait `Capturer`) ·
`src-tauri/src/capture/windows.rs` · `src-tauri/src/geometry.rs` ·
`src-tauri/src/clipboard.rs` · `src-tauri/src/overlay.rs` ·
`src/overlay/Overlay.tsx` · `src/shortcuts.ts`

**Chaîne à obtenir** : raccourci global → capture plein écran en Rust → fenêtre voile
**pré-chauffée** (créée cachée au démarrage, **jamais** créée au moment du raccourci) →
glisser un rectangle → PNG dans le presse-papier.

**Fini quand** :

1. **Latence** : 10 mesures raccourci → voile visible, **médiane < 150 ms**, journalisées.
2. **Pixel exact** : capture d'une mire de dimensions connues ; le PNG obtenu a
   exactement les dimensions du rectangle tracé, à 0 pixel près.
3. **Presse-papier** : l'image se colle dans **Paint** et dans **Discord**.
4. `Échap` referme le voile sans rien capturer.

**Risque n°1** — la latence ne passe pas. *On le verrait à* : la médiane mesurée au
point 1. **Plan B déjà arrêté** : basculer le voile — et lui seul — en fenêtre Win32
native, sans toucher au reste du projet.

**Risque n°2** — l'image passée de Rust au webview sature l'IPC. Un plein écran
1920×1080 en base64 pèse ~8 Mo par capture. *On le verrait à* : la latence du point 1
qui explose sur les grandes captures. **Parade prévue** : réponse en octets bruts
(IPC binaire de Tauri 2) ou fichier temporaire servi par le protocole `asset`, jamais
de base64.

**Risque n°3** — le voile se photographie lui-même. Constaté le 31 août 2026 sur
l'outil du cockpit. **Parade** : on capture *avant* d'afficher le voile, jamais l'inverse.

---

## Lot 2 — Registre des raccourcis + page Aide qui en DÉRIVE

**Fichiers** : `src/shortcuts.ts` (source unique) · `src/pages/Aide.tsx` ·
`src/App.tsx` · `test/shortcuts.test.ts`

Un seul tableau typé décrit chaque raccourci : combinaison, action, description,
catégorie. **Deux consommateurs, zéro recopie** : l'enregistrement réel des raccourcis,
et la page Aide.

**Fini quand** : j'ajoute une entrée au tableau, je relance, et le raccourci **apparaît
dans l'Aide sans qu'aucun fichier d'aide ait été touché**. Un test le prouve : le nombre
de lignes rendues par l'Aide est égal à la longueur du registre, et chaque combinaison
enregistrée s'y retrouve.

**Risque** : la tentation de « juste écrire la liste dans l'Aide, c'est plus rapide ».
C'est exactement le défaut qui a produit cinq copies mortes en une soirée le mois
dernier. *On le verrait à* : le test ci-dessus, qui devient rouge.

**Note** : `PrintScreen` est associé à l'Outil Capture de Windows par défaut sur
Windows 11 — **à confirmer sur cette machine**. Si le conflit existe, l'Aide doit le
dire et proposer une combinaison de repli.

---

## Lot 3 — Capture fenêtre et écran entier

**Fichiers** : `src-tauri/src/capture/windows.rs` · `src/overlay/Overlay.tsx` ·
`src/shortcuts.ts`

**Fini quand** : survol d'une fenêtre → son contour se surligne → clic → capture de
cette seule fenêtre, y compris si elle est partiellement recouverte. Vérifié sur trois
applications de natures différentes (une native, une Electron, un navigateur).

**Risque** : `Windows.Graphics.Capture` dessine une bordure jaune sur certaines
versions de Windows, et certaines applications accélérées par le GPU rendent noir avec
`PrintWindow`. *On le verrait à* : l'essai sur les trois applications.
**Non vérifié à ce jour** : quelle API des deux passe sur cette machine. À trancher
par la mesure au lot 3, pas par la documentation.

---

## Lot 4 — Éditeur d'annotation

**Fichiers** : `src/editor/*` · `src-tauri/src/export.rs`

Flèche, texte, rectangle, surlignage, **flou**.

**Fini quand** :

- Le PNG exporté d'une zone floutée **ne permet plus de relire le texte**, et le
  fichier ne contient **aucune couche réversible** — le flou est appliqué aux pixels
  à l'export, pas dessiné par-dessus.
- Annuler / Rétablir fonctionne sur 20 opérations d'affilée.

**Risque — c'est une faille, pas un défaut cosmétique** : un flou implémenté comme
calque laisse l'image d'origine dans le fichier ; une pixellisation à gros blocs peut
se remonter. *On le verrait à* : ouvrir le PNG exporté dans un éditeur tiers et
chercher la donnée d'origine. **À faire auditer par l'agent Sécurité, pas par moi.**

---

## Lot 5 — Enregistrement et bibliothèque

**Fichiers** : `src-tauri/src/library.rs` · `src/pages/Bibliotheque.tsx`

SQLite local + fichiers + vignettes. **Aucun réseau.**

**Fini quand** : une capture enregistrée se retrouve après redémarrage de l'application ;
la recherche par date fonctionne ; un « ne pas enregistrer automatiquement » existe et
est respecté.

**Risque de confidentialité** : l'historique est un dossier de captures d'écran, donc
potentiellement des mots de passe en clair sous forme d'image. Rétention et effacement
doivent exister **dès ce lot**. *On le verrait à* : l'audit Sécurité.

---

## Lot 6 — Le verre, et la fenêtre étroite

**Fichiers** : `src/styles/tokens.css` · composants d'interface

**Fini quand** : à **480×600**, tous les menus restent atteignables — constaté par
capture d'écran, pas par relecture de code. Thème clair et thème sombre tous les deux
vérifiés.

**Méthode** : skills `design-system` puis `verre-et-mobile-first` **avant** d'écrire le
CSS, puis l'agent **Relecteur UI** sur l'écran qui tourne.

**Risque** : « verre à la Apple » ≠ Acrylic de Windows. L'effet visé se compose de deux
choses distinctes : le matériau de fenêtre (`window-vibrancy` 0.8.0) **et** des panneaux
internes en `backdrop-filter`. *On le verrait à* : la revue d'interface.

---

## Lot 7 — Capture d'une page web qui défile

**Fichiers** : à définir au lot 7.

**Fini quand** : une page de documentation longue est capturée en entier, sans bande
dupliquée ni en-tête collant répété.

**Risque, et il est élevé** : il n'existe aucune API Windows pour ça. La technique est
la simulation de molette suivie d'un recollage par corrélation d'images ; elle rate sur
les listes virtualisées, les en-têtes collants et le défilement animé.
**Ce lot est le dernier pour cette raison.** Si le résultat n'est pas fiable après un
temps borné, on le livre marqué « expérimental » plutôt que de le faire passer pour
acquis.

---

## Non vérifié à ce jour — 2 septembre 2026

1. **Le comportement multi-écran et DPI mixtes.** Un seul écran sur cette machine.
   Le code sera écrit correct, il ne sera pas prouvé.
2. **La latence réelle du voile.** C'est tout l'objet du lot 1 ; aucune valeur n'est
   annoncée avant de l'avoir chronométrée.
3. **Quelle API de capture fenêtre passe** (`Windows.Graphics.Capture` ou
   `PrintWindow`). Tranché au lot 3, par la mesure.
4. **Le conflit `PrintScreen` avec l'Outil Capture de Windows** sur cette machine.
5. **Si `tauri-plugin-clipboard-manager` écrit correctement une image** dans le
   presse-papier Windows. Repli identifié : `arboard` 3.6.1 — mature (43 M de
   téléchargements) mais **sans publication depuis le 23 août 2025**, ce qui dépasse le
   seuil de 6 mois du profil et doit être signalé si on y vient.
6. **La géométrie déjà éprouvée d'AgentOS** (`normaliseRect`, `versSource`,
   `dispositionPlanche`, `verdictCapture`, 9 tests) serait réutilisable telle quelle.
   **Je ne l'ai pas lue** : elle vit hors de `F:\PROJECTS\Apps\cliche` et la consigne
   est de rester dans ce dossier. En attente d'un feu vert explicite.

## Comment on teste une application Tauri

Pas avec Playwright — il pilote un navigateur, pas une fenêtre native. Le mécanisme
prévu est **`tauri-driver`** (WebDriver). À mettre en place au lot 2, pas au lot 7.
