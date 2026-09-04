# Cliché — ce qui a été mesuré

Le registre durable des mesures. Les procédures de séance ne vivent PAS ici :
elles servent une fois et sortent du dépôt. Ce qui reste, ce sont les chiffres,
leur date, le commit sur lequel ils ont été pris, et ce que chaque étape mesure.

Toutes les mesures sont prises par Thierry, sur sa machine, avec
`CLICHE_BENCH=20` et `CLICHE_TRANSPORT=bmp`.

---

## Le contrat de mesure — ce que chaque étape signifie

**Depuis le 4 septembre 2026 (`a134cb7`), cinq étapes.** Les deux bouts du TOTAL
n'ont pas bougé depuis le lot 1 : début de la saisie d'écran → la page a accusé
qu'elle a peint une fenêtre visible.

| étape | ce qu'elle mesure |
| --- | --- |
| `capture` | la saisie de l'écran par `xcap` |
| `transport` | la mise en scène du tampon (BMP en mémoire) |
| `decoded` | `eval` → fetch → `HTMLImageElement.decode()` → accusé. **Fenêtre cachée, aucun `requestAnimationFrame`.** |
| `shown` | `show()` + `set_focus()`. **La fenêtre devient visible ICI.** |
| `painted` | une image d'animation après l'affichage, plus l'accusé |

**TOTAL n'est pas une somme.** C'est la marque la plus tardive du run, mesurée
depuis `begin_run` (`timing.rs:405`, et le test « le total est la marque la PLUS
TARDIVE »). Si la somme des étapes s'écarte du TOTAL, du temps est tombé entre
deux marques et n'est compté nulle part.

**Avant `a134cb7`, quatre étapes**, et `shown` puis `painted` ne mesuraient pas
la même chose : la fenêtre devenait visible AVANT le décodage, donc `shown`
valait ~0 et `painted` contenait tout le travail de la page. **Étape par étape,
les deux formes de rapport ne sont pas comparables. Seul le TOTAL l'est.**

### Le budget

**150 ms** au TOTAL, p95. C'est le p95 qui compte, pas la médiane.

---

## Les séances

| Date | Commit | Runs | TOTAL médiane | TOTAL p95 | Marge au p95 |
| --- | --- | --- | --- | --- | --- |
| 4 sept. 2026 | pré-poignées | 20 | 117,2 ms | 123,5 ms | 26,5 ms |
| 4 sept. 2026 | pré-poignées | 20 | 121,7 ms | 134,3 ms | 15,7 ms |
| 4 sept. 2026 (matin) | pré-poignées | 20 | 125,7 ms | 140,9 ms | **9,1 ms** |
| 4 sept. 2026 | `868ba0d` | 18 | 115,3 ms | 121,7 ms | 28,3 ms |
| 4 sept. 2026 | `a134cb7` | 19 | **112,6 ms** | **120,5 ms** | **29,5 ms** |

Les trois premières lignes précèdent le contrat à cinq étapes : leur TOTAL reste
comparable, leur détail non.

### Le détail des deux séances qui encadrent le correctif des clignotements

| étape | `868ba0d` (18 runs) | `a134cb7` (19 runs) |
| --- | --- | --- |
| `capture` | 23,4 / 25,4 | 24,3 / 26,8 |
| `transport` | 1,4 / 1,6 | 1,4 / 1,7 |
| `decoded` | — | 74,9 / 79,5 |
| `shown` | 0,0 / 0,2 | 11,8 / 13,7 |
| `painted` | 91,3 / 94,3 | 0,4 / 0,6 |
| **TOTAL** | **115,3 / 121,7** | **112,6 / 120,5** |

*(médiane / p95, en millisecondes)*

---

## Le plancher de bruit — MESURÉ, pas estimé

`capture.rs`, `geometry.rs` et `timing.rs` sont **identiques** entre `868ba0d` et
`a134cb7` (`git diff` vide sur les trois). L'étape `capture` est donc un témoin :
tout ce qu'elle bouge d'une séance à l'autre est du bruit de machine.

Elle a bougé de **+0,9 ms en médiane et +1,4 ms au p95**.

C'est l'ordre de grandeur à opposer à tout écart inférieur à ~1 ms par étape. Un
TOTAL qui varie de deux ou trois millisecondes entre deux séances ne prouve rien
à lui seul.

---

## Verdict — le correctif des clignotements (`a134cb7`)

Le TOTAL a **baissé de 2,7 ms** en médiane et de 1,2 ms au p95, alors que la
prédiction écrite avant la mesure annonçait une **hausse** d'environ un
aller-retour IPC. **La prédiction était fausse**, et voici pourquoi — aucune
étape n'a cessé d'être comptée :

1. **Rien n'est tombé entre les marques.** Somme des médianes avant :
   23,4 + 1,4 + 0,0 + 91,3 = 116,1 contre un TOTAL de 115,3. Après :
   24,3 + 1,4 + 74,9 + 11,8 + 0,4 = 112,8 contre un TOTAL de 112,6. Les deux
   sommes collent à leur TOTAL à moins d'une milliseconde.
2. **`decoded` (74,9) vaut l'ancien `painted` (91,3) moins 16,4 ms**, soit à peu
   près une image à 60 Hz (16,7 ms). C'est exactement l'image d'animation que
   l'ancien `painted` contenait et que `decoded` ne contient plus.
3. **`painted` à 0,4 ms ne peut pas contenir une image d'animation.** La seule
   lecture compatible : l'image d'animation, désormais planifiée juste après
   l'accusé de décodage, s'exécute **pendant** que Rust affiche la fenêtre. Ce
   qui était en série est devenu parallèle.

Donc le gain n'est pas un artefact comptable : c'est un recouvrement réel, plus
environ une milliseconde de bruit de séance. Ce que la prédiction avait manqué,
c'est que déplacer l'image d'animation après l'accusé la ferait chevaucher
`show()` au lieu de la précéder.

**Ce qui reste inexpliqué : `shown` est passé de 0,0 à 11,8 ms** pour les deux
mêmes appels, `show()` et `set_focus()`. Hypothèse non vérifiée : afficher une
fenêtre dont le contenu a changé pendant qu'elle était cachée oblige à construire
et présenter une première image, là où l'ancien ordre révélait une fenêtre déjà
composée. **Pour trancher il faudrait couper `shown` en deux marques**, une après
`show()` et une après `set_focus()`. Non fait : la marge au p95 est de 29,5 ms,
donc ce n'est pas urgent.

---

## Le démarrage à froid — question OUVERTE

**Le fait.** Sur `868ba0d`, les runs 1 **et** 2 n'ont pas accusé réception dans
les 3 secondes de `BENCH_RUN_TIMEOUT`. Sur `a134cb7`, seul le run 1. Depuis le
correctif d'ordre, c'est le repli à 250 ms qui rattrape cette première capture —
donc **l'utilisateur le voit à chaque démarrage**.

**Trois causes possibles, aucune vérifiée :**

- **(a)** le script de `veil.html` ne s'exécute pas tant que la fenêtre n'a
  jamais été affichée ;
- **(b)** le script s'exécute, mais `window.eval` n'atteint pas une fenêtre
  jamais affichée ;
- **(c)** les deux fonctionnent, et c'est le PREMIER `decode()` qui dépasse 3 s.

**L'instrument.** La commande `veil_ready(phase)` fait dire à la page où elle en
est. Deux phases : `loaded` au démarrage du module, `show-entered` à l'entrée de
`__clicheShow`. La seconde ne part **qu'une fois par page**, sur la première
capture — celle que le repli écarte déjà de la mesure — pour ne rien coûter aux
runs mesurés.

### Comment lire le terminal

| Ce que tu vois | Ce que ça veut dire |
| --- | --- |
| aucune ligne `ready \`loaded\`` avant le premier affichage | **(a)** ou l'IPC ne sort pas d'une fenêtre cachée |
| `loaded` au démarrage, mais pas de `show-entered` au run 1 | **(b)** — `eval` n'atteint pas une fenêtre jamais affichée |
| `loaded` **et** `show-entered`, mais pas de `veil_decoded` | **(c)** — c'est le premier décodage qui traîne |
| `show-entered` sans `loaded` | l'IPC d'une fenêtre cachée fonctionne, et c'est l'appel au chargement du module qui s'est perdu |

### La procédure — 2 minutes

```powershell
cd F:\PROJECTS\Apps\cliche
git checkout main; git pull --ff-only; git status --short   # doit être muet
pnpm tauri dev
```

**Sans `CLICHE_BENCH`** : la question porte sur la PREMIÈRE capture, pas sur
vingt. La fenêtre « Cliché » s'ouvre après 30 à 60 s de compilation.

1. Note ce que le terminal affiche **avant** toute pression de touche : y a-t-il
   une ligne `ready` ?
2. `Ctrl+Maj+2`, puis **Échap**. Ton écran sera couvert une fois.
3. Copie **toutes** les lignes `[cliche] veil:` du terminal, dans l'ordre.

Puis `Ctrl+C`.

---

## Ce qui n'est mesuré nulle part, et devrait l'être un jour

- Le coût du fil de repli (250 ms par capture, sur le fil du raccourci).
- Le surcoût de l'ACL par `invoke` : une recherche dans une table, jamais chronométrée.
- La partie du trajet qui précède l'entrée du gestionnaire de raccourci — touche
  physique, crochet bas niveau de Windows, fil de `global-hotkey`. Hors de toute
  figure imprimée. **Inconnu n'est pas zéro.**
- Le voile sur un second écran, et en DPI mixtes : une seule dalle sur la machine.
