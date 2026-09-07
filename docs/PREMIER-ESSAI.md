# Cliché — le premier essai, en deux minutes

**➜ L'installeur à télécharger : [`Cliche_0.1.0_x64-setup.exe`](https://github.com/thierryvm/cliche/releases/download/v0.1.0/Cliche_0.1.0_x64-setup.exe)** — 2 362 737 octets, sur la [release v0.1.0](https://github.com/thierryvm/cliche/releases/tag/v0.1.0).

> Pour Thierry, à faire soi-même. Écrit le 6 septembre 2026, lien de release ajouté le 7.
>
> **Le geste 1 est ACQUIS.** Thierry a installé la release v0.1.0 et lancé
> Cliché le 7 septembre 2026 vers 1 h 20 : la fenêtre s'ouvre. C'est le premier
> lancement de ce produit par un humain, et le seul point de ce document qui
> soit un compte rendu.
>
> Les gestes 2, 3 et 4 n'ont pas été rapportés dans les formes et **ne sont pas
> tenus pour faits**. Rien du reste n'a été observé : l'application n'a jamais
> été lancée par l'agent, ni installée. **Tout ce document reste une procédure à
> exécuter, pas un compte rendu.** Ce qui a été vérifié sur le binaire, sans le
> lancer, est listé à la fin.

---

## 1. Où est l'installeur

**Construit localement** (existe déjà, 2,3 Mo) :

```
F:\PROJECTS\Apps\cliche\src-tauri\target\release\bundle\nsis\Cliche_0.1.0_x64-setup.exe
```

**Produit par la CI**, sur chaque exécution du job « Windows build » : artefact
`cliche-nsis-installer`, conservé 7 jours. Pour le récupérer sans passer par le
navigateur :

```powershell
work perso -NoCd
cd F:\PROJECTS\Apps\cliche
gh run download --name cliche-nsis-installer --dir "$env:USERPROFILE\Downloads\cliche"
```

**Depuis la release publiée — c'est celui-là qu'il faut prendre.** Publiée le
7 septembre 2026, c'est exactement l'artefact produit par la CI sur le commit
`493ae0e`, pas un binaire construit sur cette machine :

<https://github.com/thierryvm/cliche/releases/download/v0.1.0/Cliche_0.1.0_x64-setup.exe>

```powershell
work perso -NoCd
gh release download v0.1.0 --pattern "*setup.exe" --dir "$env:USERPROFILE\Downloads\cliche"
```

Sa somme SHA-256 est publiée dans le corps de la release. Pour la contrôler :

```powershell
Get-FileHash "$env:USERPROFILE\Downloads\cliche\Cliche_0.1.0_x64-setup.exe" -Algorithm SHA256
```

---

## 2. Installer

Double-clic sur `Cliche_0.1.0_x64-setup.exe`.

⚠️ **Windows va afficher un avertissement SmartScreen** — « Windows a protégé
votre ordinateur ». C'est normal et attendu : **le binaire n'est pas signé**, il
n'y a pas de certificat de signature de code dans ce projet. Cliquer
« Informations complémentaires », puis « Exécuter quand même ».

C'est aussi ce que verra n'importe qui d'autre à qui tu enverrais ce fichier.

---

## 3. Trouver l'exécutable installé

Le mode d'installation par défaut de Tauri n'a **pas été vérifié** pour cette
version, donc le chemin n'est pas affirmé ici. Cette commande le trouve à coup
sûr :

```powershell
Get-ChildItem -Path "$env:LOCALAPPDATA","$env:ProgramFiles" -Filter "Cliche.exe" -Recurse -ErrorAction SilentlyContinue |
  Select-Object -First 3 -ExpandProperty FullName
```

Les deux emplacements possibles sont :

- `C:\Users\thier\AppData\Local\Cliche\Cliche.exe` (installation pour cet
  utilisateur — le plus probable)
- `C:\Program Files\Cliche\Cliche.exe` (installation pour toute la machine)

**Le plus simple reste le menu Démarrer** : touche Windows, taper `Cliche`,
Entrée.

---

## 4. Ce qu'il faut essayer — quatre gestes

### Geste 0 — TUER TOUTE INSTANCE DÉJÀ EN VIE. À faire en premier, à chaque fois.

⚠️ **Ce geste n'est nécessaire que tant que le binaire installé est antérieur au
correctif du 7 septembre 2026.** La v0.1.0 en est dépourvue ; le prochain
installeur l'aura.

Ouvrir le **Gestionnaire des tâches** (`Ctrl + Maj + Échap`), onglet
« Détails », chercher **`cliche.exe`**, et terminer **toutes** les entrées
trouvées. Il peut y en avoir **deux**, et c'est le cas intéressant.

**Pourquoi.** Sur la v0.1.0, fermer la fenêtre de Cliché ne termine pas
Cliché. Le voile — la nappe plein écran sur laquelle on trace un rectangle —
est une fenêtre créée cachée au démarrage, et le moteur de Tauri ne demande la
sortie que lorsqu'il ne reste **plus aucune** fenêtre
(`tauri-runtime-wry-2.11.4/src/lib.rs:4310-4325`). Le voile la garde ouverte.
Le processus survit donc **sans fenêtre visible, sans icône dans la barre des
tâches — et en tenant toujours `Ctrl + Maj + 2`**.

**Ce que ça donne si on l'oublie**, et c'est exactement ce qui est arrivé le
7 septembre à 1 h 27 : on relance Cliché, la nouvelle fenêtre affiche un
bandeau rouge « Raccourci refusé — Ctrl + Maj + 2 est tenu par une autre
application », et l'autre application **est Cliché**. Pire : le raccourci
fonctionne quand même, mais c'est le voile du **fantôme** qui s'affiche, pas
celui de la fenêtre qu'on regarde.

Deux `cliche.exe` dans la liste = un fantôme est là. Un seul, alors qu'aucune
fenêtre Cliché n'est ouverte = c'est un fantôme aussi.

### Geste 1 — la fenêtre s'ouvre — ✅ FAIT le 7 septembre 2026

Lancer Cliché. **Attendu** : une fenêtre avec une barre de titre sur mesure
(pas celle de Windows), le titre « Cliché » à gauche, puis **trois onglets
écrits en toutes lettres — « Capturer · Aide · Réglages » —** et enfin réduire,
agrandir, fermer.

**Un seul onglet est enfoncé à la fois**, fond plein et barre d'accent dessous.
À l'ouverture, c'est « Capturer ».

Sous le titre : **« Capturer »**, une grande tuile colorée « Capturer une zone »,
et deux tuiles grises marquées « à venir ».

> 🔴 Si la fenêtre ne s'ouvre pas du tout, ou se ferme aussitôt : note le
> message exact. C'est le défaut le plus grave possible ici.

#### 👀 La ligne du raccourci, sous les tuiles — le correctif du 7 septembre après-midi

**Attendu** : la combinaison **`Ctrl` + `Maj` + `2`** dessinée en touches de clavier.

Pendant une fraction de seconde au tout premier affichage tu peux voir une
touche **grise et vide** à la place : c'est normal, c'est le lanceur qui attend
la réponse du démarrage. Elle doit se remplir toute seule.

> 🔴 **Ce qui serait un défaut** : un bandeau rouge « Échec — Le registre des
> raccourcis n'a pas pu être lu **: …** » **qui reste**. Recopie-le **en entier,
> avec ce qui suit les deux-points** — c'est la raison technique, et c'est
> précisément ce qui manquait le 7 septembre à midi pour comprendre sans moi.

**Ce qui s'est passé, et ce que ça change.** Sur le binaire du matin, ce bandeau
rouge s'affichait alors que le raccourci fonctionnait. Cause : Tauri construit
la fenêtre principale **avant** que le code de démarrage de Cliché ne tourne
(`tauri-2.11.5/src/app.rs:2521-2535`), donc l'écran demandait l'état du
raccourci pendant que Cliché était encore en train de démarrer — et cet état
n'était posé qu'à la toute dernière ligne du démarrage, après la construction
d'une seconde fenêtre. L'état existe maintenant dès la **première** instruction,
et l'écran sait attendre au lieu de conclure à un échec.

**Ce qui n'est pas prouvé** : que ça suffise sur ta machine. Personne n'a lancé
l'application. C'est cette ligne, sur ton écran, qui tranche.

**Ce que ces onglets remplacent, et ce qu'il faut vérifier.** Jusqu'au
7 septembre 2026 il n'y avait que deux boutons ronds, ⓘ et ⚙, et rien ne disait
comment revenir : depuis l'Aide, il fallait re-cliquer le bouton déjà enfoncé.
Tu as dû double-cliquer pour retrouver l'accueil.

**Le geste** : clique « Aide », puis « Réglages », puis **« Capturer »**. Tu
dois revenir à l'accueil du premier coup, sans deviner. Si un seul de ces trois
allers-retours demande deux clics, dis-le-moi.

**Et rétrécis la fenêtre au maximum** (elle refuse de descendre sous 480 px de
large). Les trois onglets doivent rester entiers ; c'est le titre « Cliché » qui
doit rétrécir en premier. Vérifié en capture à 480 px, jamais dans une vraie
fenêtre — donc si un onglet est coupé, c'est une vraie trouvaille.

### Geste 2 — la tuile

Ouvrir une fenêtre reconnaissable (le Bloc-notes avec du texte, par exemple) et
la placer **derrière** Cliché. Puis, dans Cliché, **cliquer sur la grande tuile
« Capturer une zone »**.

**Attendu** : Cliché **disparaît**, l'écran se fige, on trace un rectangle à la
souris. Après le lâcher : un message vert « 933×577 copié » (avec tes chiffres)
s'affiche brièvement, puis tout se referme et **Cliché revient**.

Coller dans Paint (`Ctrl+V`).

> **La question qui compte** : est-ce que l'image collée contient la fenêtre de
> Cliché ? Elle ne devrait PAS — c'est exactement ce que le masquage ajouté le
> 6 septembre doit empêcher. Si Cliché apparaît dans l'image, ou apparaît
> **à moitié effacé**, l'attente de 120 ms est trop courte et il faut me le dire.

#### ⚠️ C'est CE geste qui juge le correctif du 7 septembre 2026

Thierry a rencontré ce défaut **deux fois**, sur les deux binaires : « tu vois
l'outil Cliché en transparence qui empêche toute prise de screenshot convenable
lorsque j'utilise le bouton dans l'interface de capture ». Le pari sur la durée
de l'animation de disparition de Windows était perdu.

Le correctif **coupe l'animation** au lieu de raccourcir le pari
(`DWMWA_TRANSITIONS_FORCEDISABLED`), et attend des présentations réelles avant
de photographier. Le délai de 120 ms n'a pas bougé d'une milliseconde : le clic
coûte ce qu'il coûtait, l'image ne peut que s'améliorer.

**RIEN DE TOUT CELA N'A ÉTÉ OBSERVÉ.** Personne n'a cliqué cette tuile depuis.
Le seul juge est l'image collée.

**Trois choses à me rapporter, dans cet ordre :**

1. **Au démarrage, dans le terminal**, une ligne unique qui commence par
   `[cliche] compositor:`. Copie-la telle quelle. Elle dit si Windows a accepté
   de couper les transitions, ou les a refusées, et sur quelle fenêtre. Si elle
   dit `could NOT be turned off`, le correctif n'a pas pu s'appliquer et le
   reste ne veut rien dire.

2. **L'image collée.** Cliché est-il dedans, entier, à moitié effacé, ou
   absent ? « Absent » est la seule bonne réponse.

3. **Après le collage, la ligne `[cliche] launch: tile capture, …`.** Elle porte
   maintenant deux chiffres : le temps passé en présentations réelles, et le
   sommeil qu'il est resté à faire pour atteindre le plancher. Copie-la.
   C'est la première mesure de ce mécanisme, et personne ne sait encore ce que
   les présentations coûtent.

#### Et pendant la sélection, regarde la plaque du bas

Une **plaque centrée**, d'environ 416 px de large, apparaît au bas de l'écran
une fois le rectangle tracé : « Entrée ou double-clic copie · Échap annule ».

Jusqu'au 7 septembre 2026 elle faisait **toute la largeur de l'écran** et elle
était **en anglais** — c'est le « bandeau noir » que tu n'as pas compris. Si
elle est encore pleine largeur, ou encore en anglais, le correctif n'est pas
dans ce binaire.

### Geste 3 — le raccourci

Passer sur une autre application (Cliché en arrière-plan), puis appuyer sur
**`Ctrl` + `Maj` + `2`**.

> Sur ton clavier belge AZERTY, c'est la touche qui donne `é` sans Maj, et `2`
> avec Maj. Le raccourci est enregistré sur la **touche physique**, donc c'est
> bien celle-là quelle que soit la disposition active.

**Attendu** : le même voile, la même sélection, le même message. Coller dans
Paint. Ici, Cliché n'a aucune raison d'être dans l'image.

### Geste 4 — les Réglages

Cliquer le bouton ⚙ de la barre de titre. **Attendu** : un écran « Réglages »
avec un champ « Raccourci de capture » montrant `Ctrl + Maj + 2`, et un bouton
« Modifier ».

Cliquer « Modifier », puis appuyer sur une combinaison libre (par exemple
`Ctrl` + `Maj` + `7`). **Attendu** : le champ affiche la nouvelle combinaison, et
elle **fonctionne tout de suite**, sans redémarrer. Vérifier que l'ancienne ne
fait plus rien.

Puis essayer une combinaison déjà prise par Windows (par exemple `Ctrl` + `Maj` +
`Échap`, le gestionnaire des tâches). **Attendu** : un message qui dit qu'elle
est déjà prise, et **l'ancienne combinaison reste active**. Cliché ne doit jamais
se retrouver sans aucun raccourci.

---

## 5. Le terminal, si tu lances depuis le dépôt

Lancé par `pnpm tauri dev` plutôt que par l'installeur, Cliché écrit dans le
terminal. Deux lignes valent le coup d'œil :

```
[cliche] launch: tile capture, 147.3 ms from hiding the window to perform_capture returning, 120.0 ms of it spent waiting for Windows to recompose the desktop without Cliche in it
[cliche] launch: the window is back
```

Le premier chiffre est le prix du masquage. **Il n'a jamais été mesuré** : c'est
la première fois qu'il s'affiche.

---

## 6. La re-mesure du budget — séparée, et elle presse

`.transparent(true)` a été ajouté à la fenêtre du voile le 6 septembre pour que
la confirmation puisse se poser sur un fond transparent. Sur Windows, cela change
le chemin de composition **pour toute la vie de la fenêtre**, pas seulement
pendant la confirmation.

**Conséquence : tous les chiffres `painted` de `docs/MESURES.md` décrivent un
pipeline qui n'existe plus.** Ils n'ont pas été effacés — la règle du projet est
qu'un nouveau chiffre se pose À CÔTÉ de l'ancien, jamais à sa place.

La procédure est celle de `docs/MESURES.md`, inchangée :

```powershell
cd F:\PROJECTS\Apps\cliche
$env:CLICHE_BENCH = 20
$env:CLICHE_TRANSPORT = "bmp"
pnpm tauri dev
```

⚠️ **Ton écran sera couvert vingt fois de suite.** Le banc n'attend aucune
frappe : il déclenche les captures lui-même et imprime son rapport au bout des
20 runs. Copier le tableau complet et me le donner : j'ajouterai la ligne dans
`docs/MESURES.md`, sous les précédentes, avec la date et le commit.

Le budget est **150 ms au p95**. La dernière valeur connue, sur une fenêtre
opaque, était **120,5 ms** — soit 29,5 ms de marge.

---

## 7. Ce que tu me rapportes

Court suffit :

1. La fenêtre s'ouvre — oui / non (si non : le message).
2. Geste 2 : Cliché est-il dans l'image collée ? oui / non / à moitié effacé.
3. Geste 3 : le raccourci a fonctionné — oui / non.
4. Geste 4 : le changement a pris effet sans redémarrer — oui / non. La
   combinaison refusée a bien laissé l'ancienne active — oui / non.
5. Le chiffre de la ligne `[cliche] launch:` si tu l'as vue.
6. Le tableau du banc, si tu as fait la re-mesure.
7. Tout ce qui t'a surpris, même sans rapport avec la liste.

---

## Ce qui a été vérifié sur le binaire, sans le lancer

**Le 7 septembre 2026, sur le binaire RÉELLEMENT PUBLIÉ** — l'installeur de la
release a été ouvert sans être exécuté, son flux LZMA décompressé
(10 066 843 octets) et le manifeste lu dedans : **1565 octets, 0 octet
non-ASCII**, `dpiAware` / `dpiAwareness` / `longPathAware` présents.

Le piège que ce contrôle devait éviter, et qui est mesuré : l'installeur NSIS
porte **son propre** manifeste, en clair au début du fichier, de 1297 octets.
Le chercher là aurait rendu un OK sur le mauvais fichier. L'outil est
`scripts/inspect-nsis.py`, et il a été sondé dans les deux sens.

Le 6 septembre 2026, sur `src-tauri\target\release\cliche.exe` (9,45 Mo), qui
est une AUTRE compilation — les deux sommes SHA-256 diffèrent, le build n'est
pas reproductible bit à bit :

| Contrôle | Résultat |
| --- | --- |
| Manifeste embarqué, octets non-ASCII | **0** sur 1565 octets |
| `dpiAware` / `dpiAwareness` | présents |
| `longPathAware` | présent |
| Manifeste source, octets non-ASCII | **0** sur 3893 octets |
| `productName` | `Cliche`, sans accent |
| Une seule version dans les trois fichiers | `0.1.0`, par `check-version.mjs` |

Le contrôle du manifeste n'est pas une précaution abstraite : un seul caractère
accentué dedans, **même dans un commentaire**, produit un binaire qui refuse de
démarrer avec `os error 14001`, alors que `cargo build` réussit sans un mot.

## Ce qui a été OBSERVÉ le 7 septembre 2026

Par Thierry, vers 1 h 20, depuis l'installeur de la release v0.1.0 — le premier
lancement de Cliché par un humain :

- **L'installeur installe** et **l'application démarre** : la fenêtre s'ouvre.
  Le contrôle du manifeste ci-dessus se voit confirmé sur pièce.
- Deux défauts sont sortis de ces dix minutes, tous deux corrigés depuis
  (l'un d'eux est la raison du geste 0) :
  - **le processus fantôme**, décrit au geste 0 ;
  - **une boîte vide au bas du voile**, d'environ 416 × 53 px avec une croix
    grise — noire en thème sombre pendant la sélection, blanche après. C'était
    la confirmation « largeur×hauteur copié » rendue **avant d'avoir quoi que
    ce soit à dire** : elle portait bien l'attribut `hidden`, mais
    `.c-toast-region { display: flex }` bat la règle du navigateur.
    Reproduite dans un navigateur, corrigée, et `scripts/check-hidden.mjs` la
    rattraperait aujourd'hui.

**Aucun des deux correctifs n'est dans le binaire de la release v0.1.0.** Ils
attendent le prochain installeur.

## Ce qui n'a PAS été vérifié

- **Le chemin d'installation** est annoncé au conditionnel plus haut : personne
  n'a lu l'installeur décompressé pour l'établir.
- **Le cycle de vie** — fermer la fenêtre termine bien le processus : OBSERVÉ
  par Thierry le 7 septembre 2026 au matin. Qu'un second lancement ramène le
  premier au premier plan reste, lui, lu dans le code et sous test, jamais vu.
- **Que couper les transitions de Windows suffise.** Le correctif du
  7 septembre supprime la cause probable — l'animation de disparition — mais
  personne n'a cliqué la tuile depuis. Geste 2, et lui seul.
- **`DwmFlush` ne prouve pas ce qu'on aimerait qu'il prouve.** Microsoft
  l'écrit : *« DwmFlush waits for any queued DirectX changes that were queued by
  the calling application to be drawn to the screen before returning. It does
  not flush the entire session rendering batch. »* Notre `hide()` n'est pas un
  changement DirectX que nous aurions mis en file. C'est une preuve de
  présentation, pas une preuve que la fenêtre a disparu de l'image composée —
  et c'est exactement pourquoi le plancher de 120 ms reste.
- **Ce que coûtent les présentations réelles.** Inconnu. La ligne
  `[cliche] launch: tile capture, …` du geste 2 est la première mesure.
- **La barre de titre à trois onglets** n'a jamais été vue dans une vraie
  fenêtre Tauri, seulement en navigateur. Le déplacement de la fenêtre par la
  barre et l'absence de déplacement en cliquant un onglet reposent sur un
  raisonnement lu dans le script de Tauri, jamais observé.
- **La plaque d'aide du voile** : OBSERVÉE par Thierry le 7 septembre 2026 à
  14 h 24 — centrée, en français, pendant une vraie sélection. Ce point est clos.
- **Que la course du démarrage soit vraiment fermée.** Le correctif de
  l'après-midi pose l'état dès la première instruction du démarrage, et l'écran
  re-demande tant que la réponse dit « pas encore ». Rien de tout cela n'a été
  vu tourner : aucun test de ce dépôt ne peut lancer le démarrage de Tauri, il
  faut une boucle d'événements. La ligne du raccourci au geste 1 est le seul juge.
- **Les deux chiffres de la relance** (un quart de seconde entre deux essais,
  douze essais au plus, soit au moins trois secondes) sont raisonnés, pas
  mesurés : personne n'a chronométré combien de temps ce démarrage met à décider.
- **Le binaire n'est pas signé** : SmartScreen avertira, chez toi comme chez
  n'importe qui d'autre.
- **Le budget de latence après le passage en fenêtre transparente** — section 6.
