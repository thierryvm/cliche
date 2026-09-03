# Cliché — la pile, et pourquoi celle-là

> Décision arrêtée le **2 septembre 2026**. Toute date est absolue.
> Ce document existe parce que `docs/PLAN.md` le citait avant qu'il existe.
>
> Il ne se met pas à jour tout seul : une montée de version qui change un
> arbitrage ci-dessous doit **réécrire la ligne concernée**, pas s'ajouter à la fin.

---

## Le besoin, en une phrase

Un utilitaire Windows local : raccourci → capture (zone / fenêtre / écran / page web
défilante) → annotation → presse-papier ou disque → historique consultable. Aucune
donnée ne quitte la machine.

## Les contraintes qui ont réellement décidé

| Contrainte | Valeur |
| --- | --- |
| Argent | **0 €**. Aucun service payant, aucun abonnement |
| Hébergement | **Aucun**. La machine de Thierry, rien d'autre |
| Équipe | **Une personne**, qui travaille en TypeScript et en Rust |
| Données | Des captures d'écran — donc potentiellement des mots de passe et des données personnelles, **en image**. C'est la contrainte de sécurité principale, et elle est **locale** |
| Existant | Une géométrie de sélection déjà éprouvée (9 tests) dans AgentOS, en TypeScript |
| Délai | Itératif. Le projet sert aussi de pièce de portfolio |

---

## Faits vérifiés le 2 septembre 2026 — API crates.io

| Crate | Version stable | Publiée | Licence |
| --- | --- | --- | --- |
| `tauri` | **2.11.5** | 2026-07-01 | Apache-2.0 OR MIT |
| `tauri-cli` | 2.11.4 | 2026-06-28 | — |
| `tauri-plugin-global-shortcut` | 2.3.2 | 2026-05-28 | — |
| `tauri-plugin-clipboard-manager` | 2.3.3 | 2026-08-31 | — |
| `tauri-plugin-updater` | 2.11.0 | 2026-08-31 | — |
| `window-vibrancy` | 0.8.0 | 2026-07-16 | Apache-2.0 OR MIT |
| `xcap` | 0.9.8 | 2026-08-01 | Apache-2.0 |
| `windows-capture` | 2.0.1 | 2026-08-08 | MIT |
| `arboard` | 3.6.1 | **2025-08-23** ⚠️ | MIT OR Apache-2.0 |

⚠️ **`arboard` n'a pas été publié depuis plus d'un an.** Le profil de développement
impose de signaler toute dépendance non modifiée depuis plus de 6 mois. 43 millions de
téléchargements : mature plutôt qu'abandonné, mais ce n'est pas une raison de ne pas
l'écrire. Il est le **repli**, pas le premier choix.

Sur la machine, mesuré le 2 septembre 2026 :

```
rustc 1.94.1 / cargo 1.94.1 / node v24.15.0 / pnpm 11.7.0
WebView2 152.0.4191.53 (présent)
Windows 11 Pro 10.0.26200
Écrans : 1 seul — \\.\DISPLAY1, 1920x1080 @ (0,0), échelle 100 %
```

---

## Les options, et ce que chacune ferme

### A — Tauri 2 intégral : capture en Rust, toute l'interface en webview (React + TS)

- **Coût** : deux langages dans un même produit ; une latence d'ouverture du voile à
  mesurer ; un binaire de l'ordre de la dizaine de méga-octets.
- **Ferme** : rien. La couche capture reste derrière un trait, macOS et Linux restent
  ouverts sans être promis.
- **Coût de sortie — FAIBLE.** Le cœur Rust (capture, presse-papier, encodage) ne dépend
  pas de Tauri et se rebranche sur autre chose ; l'interface React est du web réutilisable.

### B — Rust natif pur (egui, iced, ou Win32 direct)

- **Coût** : l'éditeur d'annotation, le verre translucide et l'aide intégrée deviennent
  trois chantiers à part entière, chacun sans écosystème.
- **Ferme** : de fait, l'apparence demandée et l'itération rapide sur l'interface.
- **Coût de sortie — ÉLEVÉ.** Tout est couplé à la boucle de rendu choisie.

### C — C# / WinUI 3

- **Gagne** : l'accès le plus direct aux API de capture de Windows, et le matériau Mica
  natif, sans approximation.
- **Ferme** : macOS et Linux, définitivement.
- **Coût de sortie — TOTAL.** Hors de la pile du seul mainteneur.

---

## Écartée, et par quelle contrainte

**C est écartée par la contrainte d'équipe.** Une seule personne maintient ce projet, et
elle travaille en TypeScript et en Rust. Un projet C# qu'elle ne maintiendra pas est un
projet mort — et il ne sert pas l'objectif de portfolio, qui suppose de pouvoir
l'expliquer et le faire évoluer.

**B est écartée par le brief.** Le verre translucide et une aide intégrée agréable sont
des demandes explicites, pas des ornements. En natif pur, ce sont les deux postes les
plus chers du projet.

---

## Recommandation : A, avec une porte de sortie mesurée

On construit tout en Tauri 2, **y compris le voile de sélection**. Mais :

1. Le voile est **pré-chauffé** — sa fenêtre est créée cachée au démarrage, **jamais**
   au moment du raccourci.
2. La latence raccourci → voile visible est **mesurée** au lot 1 (critère C1, médiane
   < 150 ms sur 10 tirs).
3. **Si elle ne passe pas**, on bascule **cette seule surface** en fenêtre Win32 native.
   Le reste du projet ne bouge pas. C'est l'option D évoquée le 2 septembre : le voile et
   l'éditeur ont des contraintes opposées — l'un veut la latence et le pixel, l'autre
   veut être beau et itérable — et rien n'oblige à leur imposer la même technologie.

### Pourquoi Tauri plutôt que le reste, concrètement

Les quatre plafonds rencontrés le 31 août 2026 sur l'outil de capture du cockpit AgentOS
étaient ceux du **navigateur**, pas ceux de la conception : sélecteur de source imposé,
accès aux seuls pixels affichés, voile confiné à la fenêtre, échelle réduite à 49 %.
Ils disparaissent tous en natif.

Et la géométrie déjà éprouvée (`normaliseRect`, `versSource`, `deplaceRect`,
`redimensionneRect`, `intersecteRect`) se transpose directement — voir
« Ce qu'on reprend de l'existant » plus bas.

---

## Ce qu'on reprend de l'existant — lu le 2 septembre 2026

Le fichier `apps/web/src/lib/capture.ts` d'AgentOS a été lu, en lecture seule, avec
l'accord explicite de Thierry. **Environ la moitié se transpose, l'autre non.**

**Transposable en Rust, et ce sont les parties qui comptent :**

| Fonction | Ce qu'elle règle |
| --- | --- |
| `normaliseRect` | Un glisser de bas en haut ou de droite à gauche. Sans elle, le geste naturel est celui qui ne marche pas |
| `versSource` | Coordonnées **affichées** → pixels **source**. C'est exactement la conversion DPI, et **l'erreur ne se voit pas sur un écran 1:1** — donc invisible sur cette machine |
| `deplaceRect`, `redimensionneRect`, `POIGNEES` | La zone reste **ajustable avant d'être prise** : un glisser raté ne se paie pas d'un nouveau geste |
| `intersecteRect` | Reclipper une zone quand la source change de taille, plutôt que l'abandonner |
| `selectionExploitable` | Un clic n'est pas une zone. C'est le cas d'échec 6 du PRD |
| `capteEnSeMasquant` | **On capture AVANT d'afficher le voile.** Le 31 août 2026, l'outil se photographiait lui-même |

**Non transposable** : `verdictCapture`, `MESSAGE_SELECTEUR_MUET`, `MESSAGE_API_ABSENTE`
et `attenteDePeinture` traitent des échecs de `getDisplayMedia`, une API de navigateur
qui n'existe pas ici. `encodeSousLaLimite` et `poidsDecode` servent un plafond de taille
imposé par un carnet distant : il n'y a pas de plafond sur un fichier local.

**Conséquence** : c'est une **transposition**, pas un copier-coller. Les 9 tests
d'origine ne se recompilent pas ; les cas qu'ils couvrent doivent être réécrits en Rust.

---

## Sécurité — la surface est petite, les deux vrais points ne le sont pas

Pas d'authentification, pas de réseau, pas de secret : la surface d'attaque est
minuscule. Restent deux points, et ils sont sérieux.

1. **Le flou doit être destructif.** Un flou appliqué comme calque au-dessus de l'image
   laisse les pixels d'origine dans le fichier. Une pixellisation à gros blocs peut se
   remonter. C'est une **faille**, pas un défaut cosmétique — critère C4.
2. **L'historique est un dossier de captures d'écran**, donc potentiellement des mots de
   passe en clair, sous forme d'image. Rétention, effacement réel et « ne pas enregistrer
   automatiquement » existent **dès le lot 5**, pas après.

Ces deux points seront audités par l'agent **Sécurité** sur le code écrit. Un audit rendu
par celui qui a décidé la pile n'est pas un audit.

---

## Risque assumé

**La latence d'ouverture du voile est la contrainte la moins bien servie par Tauri.**
Elle est tenue parce qu'elle est **mesurable dès le lot 1** et qu'un plan B existe qui ne
jette que le voile, pas le projet.

## Ce qui n'est pas décidé ici

- Le mécanisme de capture d'une **fenêtre** (`Windows.Graphics.Capture` ou `PrintWindow`)
  — tranché au lot 3, **par la mesure**, pas par la documentation.
- Le mécanisme de **presse-papier** (`tauri-plugin-clipboard-manager` en premier choix,
  `arboard` en repli) — tranché au lot 1, par l'essai.
- La **mise à jour automatique** — arbitrée séparément dans `docs/UPDATES.md`.
  Décision : pas d'updater aujourd'hui. Son adoption ferait entrer `reqwest` dans l'arbre
  de dépendances, ce qui change la posture de sécurité du projet.

---

## Portabilité — état RÉEL, vérifié le 3 septembre 2026

Contrainte posée par Thierry ce jour-là : Cliché doit à terme tourner sous Windows,
macOS et Linux. Windows d'abord, mais **rien ne doit fermer la porte**.

Bonne nouvelle : la contrainte est presque gratuite aujourd'hui, et c'est le verdict
du lot 1d qui l'a rendue telle — le voile est resté en Tauri, donc **aucun moteur
natif n'a été introduit**.

### Ce qui est DÉJÀ portable — constaté, pas supposé

| Élément | Constat |
| --- | --- |
| Notre source Rust | **Zéro** `cfg(windows)`, zéro `winapi`, zéro appel Win32 direct |
| `build.rs` | Le manifeste DPI est déjà sous `#[cfg(windows)]` / `#[cfg(not(windows))]` — le motif « une interface, un moteur par système » y est déjà appliqué |
| `xcap` 0.9.8 | Multiplateforme : dépendances `objc2*` sous macOS, `libwayshot`/`pipewire` sous Linux, toutes conditionnées par cible |
| `tauri-plugin-global-shortcut`, `tauri-plugin-clipboard-manager` | Multiplateformes |
| `image` | Rust pur |
| Transport du voile (BMP + schéma personnalisé) | Aucune API système ; le format et le protocole sont ceux de Tauri |

### Ce qui reste lié à Windows aujourd'hui

1. **Le manifeste DPI** (`windows-app-manifest.xml`) — déjà conditionné, sans effet
   ailleurs. macOS et Linux ont leurs propres mécanismes d'échelle, à traiter le jour
   du portage.
2. **La CSP du schéma personnalisé.** Sur Windows, WebView2 sert un schéma en
   `http://<schéma>.localhost` ; ailleurs c'est la forme `<schéma>:`. Le fichier porte
   désormais **les deux**, exactement comme l'entrée préexistante `asset:
   http://asset.localhost` le fait déjà pour le protocole d'assets. Précédent suivi,
   pas inventé.
3. **Le comportement de la fenêtre de voile** (plein écran, sans décoration, au-dessus
   de tout) diffère par système — sur macOS notamment, la barre de menus et les Spaces
   demandent un traitement propre. **Non éprouvé**, faute de machine.
4. **`xcap` sous Linux** passe par les portails Wayland (`pipewire`) : c'est une
   dépendance d'exécution, pas de compilation. **Non éprouvé.**

### La règle, pour tout ce qui viendra

Si une capacité devient native : **une interface unique, un moteur par système, et un
plan de portage écrit à côté**. Une bibliothèque Windows seule ne peut pas être le seul
chemin d'une capacité. Un moteur natif sans son plan de portage n'est pas fini.

**Ce qui n'est pas prouvé et ne doit pas se lire comme tel** : rien n'a jamais été
compilé ni exécuté sous macOS ou Linux. Tout ci-dessus est de la lecture de sources et
de configuration. La première compilation ailleurs révélera des choses.
