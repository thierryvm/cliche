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
| Système visuel | **Lot D, AVANT les écrans** — déplacé depuis le lot 6 | Demande du 2 sept. : le verre et la fenêtre étroite se décident avant, pas après |
| Mise à jour auto | **Aucune pour l'instant**, opt-in **avant la première copie distribuée** | Arbitré dans `docs/UPDATES.md` : un updater fait sortir des requêtes. Le déclencheur a été resserré le 2 sept. : un updater ne peut mettre à jour qu'un binaire qui le contient déjà, donc « quand il y aura des utilisateurs » est trop tard |
| Dépôt GitHub | **Public**, sous `thierryvm` | Mesuré le 2 sept. : sur ce compte, la protection de branche rend `403 — Upgrade to GitHub Pro or make this repository public`. Privé et branche protégée sont incompatibles ici |
| Licence | **PolyForm Noncommercial 1.0.0**, tranchée le 3 sept. | Asymétrie : une version publiée en licence permissive le reste pour toujours ; passer de celle-ci à MIT coûte un fichier, l'inverse est impossible. Le code reste lisible et forkable pour le portfolio, l'usage commercial reste une décision. Coût assumé : PolyForm n'est pas approuvée OSI |
| CI | `windows-latest` uniquement, 2 jobs : `Quality gates` + `Windows build` | La couche capture est irréductiblement Windows ; un job Linux vert ne prouverait rien. Gratuit et sans plafond sur un dépôt public |
| Numéro de version | **Trois fichiers, un contrôle** — `scripts/check-version.mjs` | Aucun mécanisme de source unique n'a été vérifié pour cette version de Tauri ; on ne suppose pas, on contrôle. Détail dans `docs/RELEASES.md §3` |

### Les documents du projet, et qui répond à quoi

| Document | La question à laquelle il répond |
| --- | --- |
| `docs/PRD.md` | **Pour qui**, dans quelles situations, et à quoi on reconnaît que c'est réussi |
| `docs/STACK.md` | **Avec quoi** on construit, et ce que chaque option aurait fermé |
| `docs/UPDATES.md` | **Faut-il** une mise à jour automatique, et ce qu'elle ferait sortir de la machine |
| `docs/RELEASES.md` | **Comment** on publie une version, et ce qui est déjà prêt pour le jour de l'updater |
| `docs/PLAN.md` (ici) | **Dans quel ordre** on construit, et ce qui pourrait casser |

### Distribution — ce qui est prêt, et ce qui reste à activer

Rien de ceci ne bloque un lot. C'est l'infrastructure autour du code, tenue à
jour au fil de l'eau plutôt qu'en fin de projet.

| Prêt, vérifié le 2 septembre 2026 | Reste à activer |
| --- | --- |
| Dépôt git local, 3 commits, arbre propre | Le dépôt distant — **en attente du GO de Thierry** |
| `.github/workflows/ci.yml` — chaque commande vérifiée localement | La CI n'a **jamais tourné sur GitHub** : son orchestration reste une hypothèse |
| `scripts/check-version.mjs` — testé dans les deux sens (0 et 1) | — |
| `docs/RELEASES.md` — procédure de publication complète | La première release réelle |
| `docs/UPDATES.md §7.2` — les 11 emplacements de l'updater | La paire de clés, délibérément non générée : `docs/RELEASES.md §6` |
| `capabilities/` vide de plugins, pour que l'ajout de l'updater soit visible en une ligne de diff | — |

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

## Lot D — Le système visuel, et sa page vitrine

**Déplacé ici depuis le lot 6 le 2 septembre 2026**, à la demande de Thierry, et il a
raison : le verre translucide et l'utilisabilité en fenêtre étroite ne sont pas une
finition. Ce sont des décisions de structure. Les prendre après avoir écrit cinq écrans,
c'est réécrire cinq écrans.

**Fichiers** : `src/design/tokens.css` (les jetons, source unique) ·
`src/design/glass.css` · `src/design/components.css` ·
`src/pages/SystemeVisuel.tsx` (la vitrine) · `src/App.tsx` (la route)

**Ce que ça produit, concrètement** : une page **ouvrable dans l'application**, qui
montre chaque jeton et chaque composant **avec tous ses états** — repos, survol, focus
clavier, actif, désactivé, chargement, erreur, vide. Pas une capture d'écran de
maquette : la vraie page, avec les vrais styles, celle qui casse si un jeton change.

**Fini quand** :
1. La page `/design` s'ouvre dans l'application et liste les jetons **lus depuis le CSS**,
   pas recopiés à la main.
2. Chaque composant y apparaît dans **tous** ses états, y compris le focus clavier.
3. Quatre captures : **375 px** et **1440 px**, en **thème clair** et **thème sombre**.
4. À **480×600** (la taille minimale déclarée dans `tauri.conf.json`), rien n'est coupé
   et aucun défilement horizontal n'apparaît.
5. Le contraste du texte sur verre est **mesuré** contre le fond le plus clair ET le plus
   sombre qui puisse passer derrière : ≥ 4,5:1. Mesuré, pas regardé.

**Risque n°1** — « verre à la Apple » ≠ Acrylic de Windows. L'effet se compose de deux
choses distinctes : le matériau de fenêtre (`window-vibrancy` 0.8.0) **et** des panneaux
internes en `backdrop-filter`. *On le verrait à* : la revue d'interface.

**Risque n°2 — celui qui coule le verre** : du texte lisible sur un fond qui bouge. Un
`backdrop-filter` seul rend un rectangle laiteux et illisible. La parade est écrite dans
la compétence `verre-et-mobile-first` : cinq couches, un repli `@supports`, un repli
`prefers-reduced-transparency`, et un plafond de trois surfaces de verre par écran.

**Méthode** : compétences `design-system` puis `verre-et-mobile-first` **avant** d'écrire
le CSS, agent **Design** pour l'écrire, agent **Relecteur UI** pour le relire sur l'écran
qui tourne — jamais l'auteur.

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

1. **Latence** : **20** mesures raccourci → voile visible, **médiane < 150 ms**, plus
   le p95, journalisées. *Relevé le 3 sept. en écrivant l'instrument : 10 suffit
   pour une médiane, mais avec la méthode du rang le plus proche `ceil(0,95 × 10) = 10`,
   donc sur 10 runs le p95 EST le maximum et n'apprend rien de plus que « le pire de
   dix ». Le seuil monte à 20 pour que le p95 soit un percentile et pas une décoration.*
2. **Pixel exact** : capture d'une mire de dimensions connues ; le PNG obtenu a
   exactement les dimensions du rectangle tracé, à 0 pixel près.
3. **Presse-papier** : l'image se colle dans **Paint** et dans **Discord**.
4. `Échap` referme le voile sans rien capturer.

**Risque n°1** — la latence ne passe pas. *On le verrait à* : la médiane mesurée au
point 1. **Plan B déjà arrêté** : basculer le voile — et lui seul — en fenêtre Win32
native, sans toucher au reste du projet.

### VERDICT DU RISQUE N°1 — mesuré le 3 septembre 2026 : **Tauri tient, le plan B ne sert pas**

20 runs par transport, 1920×1080 à 100 %, un seul écran, dépendances optimisées et
notre code en debug. `t0` = entrée du gestionnaire.

| Étape | **A — protocole `cliche:`, BMP** | B — `data:`, PNG + base64 |
| --- | --- | --- |
| `capture` | 22,7 ms | 24,6 ms |
| `transport` | **1,5 ms** | 83,2 ms |
| `shown` | 0,0 ms | 0,0 ms |
| `painted` | 97,6 ms | **59,6 ms** |
| **TOTAL médiane** | **122,4 ms ✅** | 167,2 ms ❌ |
| **TOTAL p95** | **133,1 ms ✅** | 180,6 ms ❌ |

**Transport A retenu.** L'arbitrage tient en une ligne : le PNG gagne 38 ms à la
peinture (867 Ko à décoder contre 8,29 Mo) et en perd 82 à l'encodage. Compresser
une image qui ne quitte jamais la machine et qu'on jette une seconde plus tard
coûte plus cher que de la transporter telle quelle.

**Trois réserves, pour que ce verdict ne se lise pas plus large qu'il n'est** :
1. Le total est une **borne supérieure** : il porte le retour IPC de l'accusé de
   peinture. Mais il s'arrête à un `requestAnimationFrame`, pas à la présentation
   par le compositeur — l'erreur existe dans les deux sens, elle n'est pas bornée.
2. **Notre code est encore en debug** (seules les dépendances sont optimisées). Le
   binaire livré sera donc *plus rapide* que 122,4 ms : le verdict est pessimiste.
3. Mesuré sur **un écran à 100 %**. Rien n'est prouvé en multi-écran ni en DPI mixte.

La marge est de 27,6 ms sur la médiane. Elle n'est pas confortable : toute étape
ajoutée entre le raccourci et la peinture se prend dessus.

### Confirmé par Thierry en séance, le 4 septembre 2026 — après 1e et 1f

Deux passes de 20 runs, transport A, sur sa machine, **avec la sélection et le
presse-papier en place** :

| Passe | TOTAL médiane | TOTAL p95 |
| --- | --- | --- |
| 1 | **117,2 ms** | 123,5 ms |
| 2 | **121,7 ms** | 134,3 ms |

Les étapes ajoutées par 1e et 1f **n'ont rien coûté** : elles arrivent après la
peinture. Le p95 de la seconde passe, **134,3 ms**, laisse **15,7 ms** de marge —
c'est le chiffre à surveiller, pas la médiane.

Ce que la même séance a **infirmé** : le voile était **invisible à l'ouverture**.
Il affiche une copie pixel-exacte de l'écran, donc rien ne distinguait l'état armé.
Corrigé par un cadre bi-ton au bord de l'écran, dont le coût est **borné par la
géométrie** (1,15 % des pixels d'un assombrissement plein écran) et **non mesuré** —
personne n'a encore relancé le banc avec ce cadre.

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

## Lot 6 — Le système visuel APPLIQUÉ à tous les écrans

Le système lui-même est décidé au **lot D**, avant les écrans. Ce lot-ci n'invente rien :
il vérifie que les écrans construits entre-temps (Aide, éditeur, bibliothèque) emploient
réellement les jetons, et il rattrape ceux qui ont dérivé.

**Fichiers** : les écrans écrits aux lots 2, 4 et 5 · `src/design/*` en lecture

**Fini quand** :
1. Une recherche de valeurs en dur (couleur hexadécimale, taille en pixels, rayon) dans
   les composants ne rend **rien** hors de `src/design/`. Le système existe à un seul
   endroit ou il n'existe pas.
2. Chaque écran est vu à **480×600** en thème clair et sombre : tous les menus
   atteignables, aucun défilement horizontal — **constaté par capture, pas par relecture
   de code**.
3. La revue de l'agent **Relecteur UI** est passée sur chaque écran.

**Risque** : la dérive silencieuse. Un `#2b2b2b` posé « juste pour ce cas » pendant le
lot 4 ne se voit pas à l'œil et casse le thème clair. *On le verrait à* : le point 1,
qui est une recherche, pas un avis.

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
6. ~~La géométrie d'AgentOS serait réutilisable telle quelle.~~ **RÉSOLU le 2 septembre
   2026** : Thierry a donné le feu vert, `apps/web/src/lib/capture.ts` a été lu en lecture
   seule. Verdict corrigé — **environ la moitié se transpose**, et « telle quelle » était
   faux : c'est du TypeScript qui doit être réécrit en Rust, et les 9 tests d'origine ne
   se recompilent pas. Le détail de ce qui se reprend et de ce qui ne se reprend pas est
   dans `docs/STACK.md`, section « Ce qu'on reprend de l'existant ».
7. **Trois lignes vides sont apparues dans ce fichier** le 2 septembre 2026 à 22:57:39,
   pendant l'exécution de deux sous-agents en parallèle, **sans auteur identifié**.
   Constaté : +3 octets pour +3 lignes, donc aucun caractère de texte ajouté ni retiré ;
   tableau des décisions, liste ci-dessus et section « Comment on teste » relus et
   identiques. Vraisemblablement un normaliseur de markdown. Le dépôt git a été initialisé
   dans la foulée : à partir du commit `1987d16`, une modification sans auteur devient
   visible dans un diff au lieu de se deviner à l'octet près.

## Comment on teste une application Tauri

Pas avec Playwright — il pilote un navigateur, pas une fenêtre native. Le mécanisme
prévu est **`tauri-driver`** (WebDriver). À mettre en place au lot 2, pas au lot 7.
