# Cliché — le premier essai, en deux minutes

> Pour Thierry, à faire soi-même. Écrit le 6 septembre 2026.
>
> C'est la première fois que Cliché sort du dépôt. Rien de ce qui suit n'a été
> observé sur une machine : l'application n'a jamais été lancée par l'agent, ni
> installée. **Tout ce document est une procédure à exécuter, pas un compte
> rendu.** Ce qui a été vérifié sur le binaire, sans le lancer, est listé à la
> fin.

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

**Depuis une release publiée** (quand le tag `v0.1.0` existera) :

```powershell
gh release download v0.1.0 --pattern "*setup.exe" --dir "$env:USERPROFILE\Downloads\cliche"
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

### Geste 1 — la fenêtre s'ouvre

Lancer Cliché. **Attendu** : une fenêtre avec une barre de titre sur mesure
(pas celle de Windows), le titre « Cliché » à gauche, et à droite deux boutons
ronds — ⓘ (aide) et ⚙ (réglages) — puis réduire, agrandir, fermer.

Sous le titre : **« Capturer »**, une grande tuile colorée « Capturer une zone »,
et deux tuiles grises marquées « à venir ».

> 🔴 Si la fenêtre ne s'ouvre pas du tout, ou se ferme aussitôt : note le
> message exact. C'est le défaut le plus grave possible ici.

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

Le 6 septembre 2026, sur `src-tauri\target\release\cliche.exe` (9,45 Mo) :

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

## Ce qui n'a PAS été vérifié

- **Que l'application démarre.** Elle n'a jamais été lancée, ni depuis le dépôt,
  ni depuis l'installeur.
- **Que l'installeur installe.** Il n'a jamais été exécuté, et son chemin
  d'installation est donc annoncé au conditionnel plus haut.
- **Le binaire n'est pas signé** : SmartScreen avertira, chez toi comme chez
  n'importe qui d'autre.
- **Le budget de latence après le passage en fenêtre transparente** — section 6.
- **Que 120 ms suffisent** pour que Windows ait fini de redessiner l'écran sans
  Cliché — c'est le geste 2 qui tranche, et lui seul.
