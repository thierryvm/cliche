# Séance de vérification du lot 1 — procédure pour Thierry

**Écrit le 4 septembre 2026.** Une seule séance, dans l'ordre, sans rien à deviner.

Ce que cette séance prouve, et que **rien d'automatisé ne peut prouver à ta place** :

| # | Ce qu'on vérifie | Pourquoi ça ne peut être que toi |
| --- | --- | --- |
| 1 | La marge de 27,6 ms tient après 1e et 1f | l'écran clignote 20 fois — je ne lance rien de visible sans ton accord |
| 2 | La sélection existe vraiment à l'écran | **jamais vue** : aucun rectangle n'a jamais été affiché |
| 3 | L'image arrive dans le presse-papier | **jamais testé** : l'aller-retour n'a pas pu être écrit (voir §6) |
| 4 | Un clic n'écrase pas ton presse-papier | exigence du PRD, cas 6 |

**Durée totale : environ 8 minutes.** Compilation comprise.

---

## Avant de commencer

Ferme ce qui pourrait gêner. Garde **Paint** et **Discord** ouverts, tu en auras besoin
aux étapes 4 et 5.

**Ton presse-papier va être écrasé** à l'étape 3. Si tu y as quelque chose, colle-le
quelque part maintenant.

Ouvre **un** terminal PowerShell :

```powershell
cd F:\PROJECTS\Apps\cliche
```

---

## Étape 1 — Lancer, avec la mesure

```powershell
$env:CLICHE_BENCH = '20'; $env:CLICHE_TRANSPORT = 'bmp'; pnpm tauri dev
```

**Ce qui se passe :** 30 à 60 secondes de compilation, puis la fenêtre « Cliché »
s'ouvre. Ensuite **ton écran est recouvert 20 fois de suite par le voile**, à peu près
une fois par seconde. **C'est normal, ça dure une quarantaine de secondes, ne touche à
rien pendant ce temps.**

**Où lire :** dans le terminal. Tu dois voir passer, dans cet ordre :

```
[cliche] startup: 1 display(s) detected
[cliche] veil: preheated, hidden, 1920x1080 physical px at (0, 0)
[cliche] shortcut: press Ctrl+Shift+Digit2
```

⚠️ **Si tu vois une ligne contenant `FAILED to take`**, le raccourci est déjà pris par
un autre logiciel. L'application tourne quand même, mais la suite ne marchera pas :
dis-le-moi, ferme le coupable, et on reprend.

Puis, après les 20 clignotements :

```
[cliche] bench: finished
[cliche] bench: transport A - custom protocol, BMP (header + memcpy)
[cliche] timing report over 20 run(s)
[cliche]   #1 capture              median   XX.X ms  p95   XX.X ms  (20 sample(s))
[cliche]   #2 transport            median    X.X ms  p95    X.X ms  (20 sample(s))
[cliche]   #3 shown                median    X.X ms  p95    X.X ms  (20 sample(s))
[cliche]   #4 painted              median   XX.X ms  p95   XX.X ms  (20 sample(s))
[cliche]   TOTAL                 median  XXX.X ms  p95  XXX.X ms
```

**Ce que je veux savoir :** la valeur de `TOTAL median`.

| Ce que tu lis | Ce que ça veut dire |
| --- | --- |
| **≤ 130 ms** | la marge tient, 1e et 1f n'ont rien coûté |
| **entre 130 et 150** | ça passe encore, mais la marge a fondu — dis-le-moi |
| **> 150 ms** | quelque chose a régressé depuis les 122,4 ms du 3 septembre |

**Ne ferme pas le terminal ni l'application** : tout le reste se fait dans cette même
session.

---

## Étape 2 — Voir la sélection vivre

C'est la première fois qu'un être humain regarde cet écran.

1. Appuie sur **`Ctrl` + `Maj` + `2`**.
2. L'écran se fige sous un voile.
3. **Glisse la souris** pour tracer un rectangle — prends une zone bien visible et
   reconnaissable, disons 300 sur 200 pixels, sur quelque chose que tu identifieras en
   la collant (une fenêtre, du texte).
4. **Relâche.**

**Regarde, pendant que tu glisses :**

- le rectangle a-t-il un **trait à deux tons** (un filet blanc doublé d'un noir) ?
  C'est la signature du produit, et la seule construction dont la lisibilité est mesurée
  (4,58:1 sur n'importe quel fond) ;
- l'extérieur du rectangle **s'assombrit-il** quand tu commences à glisser ? (Il ne doit
  **pas** s'assombrir à l'ouverture du voile — c'est le choix qu'on a gardé.)
- les **8 poignées** apparaissent-elles aux coins et aux milieux des côtés ?
  ⚠️ **Elles ne redimensionnent pas** — c'est connu, fiché, et hors du lot 1.

**Où lire :** dans le terminal, **deux** lignes doivent apparaître au relâchement.

La première est la plus intéressante — c'est celle qui trahirait une erreur de
conversion, parce qu'elle donne l'origine **et** le facteur d'échelle :

```
[cliche] veil: run N selection 300x200 physical px at (x, y) - 240000 byte(s), from a CSS rectangle at scale 1.00
```

Puis :

```
[cliche] clipboard: run N copied 300x200 physical px (240000 RGBA byte(s)) in X.X ms - OUTSIDE the 150 ms budget, which ends at `painted`
```

**Trois choses à vérifier sur ces lignes :**

1. les **dimensions** correspondent à ce que tu as tracé ;
2. l'**origine `(x, y)`** correspond à l'endroit où tu as commencé le glisser ;
3. `scale 1.00` — c'est l'échelle de ton écran. Sur une machine à 125 %, la conversion
   entrerait vraiment en jeu ; ici elle est neutre, et **c'est pour ça qu'un bug de
   conversion serait invisible chez toi**. Les tests l'éprouvent à 1,25 / 1,5 / 2,0
   faute d'écran pour le faire.

---

## Étape 3 — Le collage dans Paint

Le voile s'est refermé tout seul. **Sans rien copier d'autre entre-temps** :

1. Va dans **Paint**.
2. `Ctrl` + `V`.

**Ce que tu dois voir :** l'image de ta zone, **aux bonnes dimensions**, avec les bonnes
couleurs.

**Ce qui serait un défaut :**

| Symptôme | Ce que ça signifie |
| --- | --- |
| rien ne se colle | l'image n'atteint pas le presse-papier — **c'est le trou n°1** |
| les couleurs sont inversées (rouge ↔ bleu) | ordre des canaux RGBA/BGRA |
| l'image est décalée d'un pixel ou deux | conversion CSS → physique |
| l'image est déformée ou penchée | erreur de pas de ligne (stride) |

Note les dimensions que Paint affiche en bas — elles doivent égaler la ligne du terminal.

---

## Étape 4 — Le collage dans Discord

Toujours **sans rien recopier** :

1. Va dans **Discord**, dans un salon où tu peux effacer (ou tes messages privés).
2. `Ctrl` + `V`.

**Ce que tu dois voir :** un aperçu de l'image, prêt à envoyer. **N'envoie pas**, ça
suffit à prouver que Discord accepte le format.

Paint **et** Discord, parce qu'ils ne lisent pas le presse-papier de la même façon :
l'un accepte un format que l'autre refuse est un cas réel, et c'est pour ça que le
critère du 2 septembre demandait les deux.

---

## Étape 5 — Le clic ne doit RIEN écraser

C'est l'exigence du PRD, cas 6 : *« ni presse-papier écrasé par une image vide »*.

1. Appuie sur **`Ctrl` + `Maj` + `2`**.
2. **Fais un simple clic** — appuie et relâche sans bouger.
3. Le voile doit **rester ouvert** (tu peux retracer sans relancer le raccourci).
4. Appuie sur **`Échap`** pour fermer.
5. Retourne dans **Paint**, `Ctrl` + `V`.

**Ce que tu dois voir : la MÊME image qu'à l'étape 3.** Si c'est le cas, le clic n'a pas
touché ton presse-papier, et l'exigence est tenue.

**Où lire :** le terminal doit afficher `[cliche] veil: dismissed` à l'appui sur Échap,
et **aucune** ligne `clipboard:` pour le clic.

Le seuil est de **64 pixels d'aire** (un carré de 8 × 8). En dessous, c'est un clic ; au
dessus, c'est un geste. Si un vrai petit rectangle volontaire s'est fait refuser, dis-le-moi :
le seuil est discutable, il est écrit et il se change.

---

## Pour finir

`Ctrl` + `C` dans le terminal. Puis :

```powershell
Remove-Item Env:\CLICHE_BENCH, Env:\CLICHE_TRANSPORT
```

Sans ça, le prochain `pnpm tauri dev` dans **ce même terminal** relancerait 20
clignotements.

---

## Ce que je veux que tu me rapportes

Cinq choses, dans l'ordre :

1. La valeur de **`TOTAL median`** de l'étape 1.
2. La ligne **`clipboard:`** de l'étape 2 (copier-coller).
3. **Paint** : l'image est arrivée, oui ou non — et ses dimensions.
4. **Discord** : l'aperçu est arrivé, oui ou non.
5. **Étape 5** : la même image est revenue, oui ou non.

Si quelque chose échoue, **le texte brut du terminal** vaut mieux qu'une description.

---

## §6 — Le trou que cette séance vient combler

L'aller-retour par le presse-papier **n'existe pas dans la suite de tests**, et ce n'est
pas un oubli. Un test réel exige un `Clipboard<R>`, que seul le plugin distribue via
l'état managé, donc une `App`, donc le *feature* `test` de Tauri. **Mesuré le
4 septembre 2026** : ajouter ce feature compile, puis le binaire de test **refuse de
démarrer** (`STATUS_ENTRYPOINT_NOT_FOUND`), avant le premier test, et toute la suite
meurt. Isolé en n'annulant que cette ligne : le même arbre passe alors 97 tests.

Le test a donc été **retiré** plutôt que remplacé par un substitut qui passerait sans
jamais toucher un presse-papier.

**Conséquence directe : tant que tu n'as pas collé, « l'image arrive dans le
presse-papier » est une affirmation non testée.** Ton `Ctrl+V` est la seule preuve qui
existe.
