# Mesure de référence avant le correctif des clignotements

**Écrit le 4 septembre 2026.** Courte : environ **5 minutes**, dont 40 secondes
pendant lesquelles ton écran clignote 20 fois.

## Pourquoi maintenant, et pas après le correctif

Ta mesure du matin — **125,7 ms de médiane, 140,9 de p95** — a été prise sur un
`main` qui n'avait **ni les poignées ni les deux correctifs presse-papier**.
`main` a bougé trois fois depuis. Comparer le futur correctif à ce chiffre-là
reviendrait à mesurer deux changements et à en attribuer le coût à un seul.

Et le nombre qui compte n'est plus la médiane : **la marge au p95 est de
9,1 ms** (140,9 contre un budget de 150). C'est le chiffre le plus serré du
projet. Il mérite d'être connu sur le code d'aujourd'hui avant qu'on empile
quoi que ce soit dessus.

---

## Étape 1 — Se placer sur le vrai `main`

```powershell
cd F:\PROJECTS\Apps\cliche
git checkout main
git pull --ff-only
git status --short          # doit ne RIEN afficher
git log --oneline -1
```

Le dernier commit doit être **`security(acl): declare every command and cap each
window`**, et `git status` doit être muet.

*Ce document listait d'abord trois SHA précis. Ils sont périmés : `main` a reçu
depuis le manifeste ACL. Un numéro recopié à la main dans une procédure vieillit
en silence — ce qui compte vraiment, c'est que ton arbre soit propre et à jour
sur `origin/main`, pas qu'il porte un numéro que j'ai tapé un jour.*

Si `git status` affiche quoi que ce soit, arrête-toi et dis-le-moi : la mesure
porterait sur autre chose que ce qu'on croit.

### Si le voile ne s'affiche PLUS DU TOUT

Nouveau risque depuis le manifeste ACL, et il faut que tu saches le reconnaître.
L'application déclare maintenant, commande par commande, quelle fenêtre a le
droit de l'appeler. C'est vérifié par des tests qui interrogent la vraie table
de décision — mais **jamais par une exécution réelle** : personne n'a lancé
Cliché depuis ce changement.

Le symptôme, s'il s'était trompé : rien ne se passe au raccourci, ou l'écran se
fige sans jamais rendre la main, et le terminal montre une ligne contenant
`not allowed by ACL`.

Dans ce cas, ne cherche pas : arrête tout et lance

```powershell
git log --oneline -1        # note le SHA, je veux le message exact
```

puis dis-le-moi. Ne mesure rien : le chiffre ne voudrait rien dire.

## Étape 2 — Les 20 mesures

```powershell
$env:CLICHE_BENCH = '20'; $env:CLICHE_TRANSPORT = 'bmp'; pnpm tauri dev
```

**Ce qui se passe :** 30 à 60 secondes de compilation, la fenêtre « Cliché »
s'ouvre, puis **ton écran est recouvert 20 fois de suite**, environ une fois par
seconde. Ne touche à rien pendant ces quarante secondes.

**À lire dans le terminal**, le dernier bloc :

```
[cliche] bench: finished
[cliche] bench: transport A - custom protocol, BMP (header + memcpy)
[cliche] timing report over 20 run(s)
[cliche]   #1 capture     median  XX.X ms  p95  XX.X ms
[cliche]   #2 transport   median   X.X ms  p95   X.X ms
[cliche]   #3 shown       median   X.X ms  p95   X.X ms
[cliche]   #4 painted     median  XX.X ms  p95  XX.X ms
[cliche]   TOTAL        median XXX.X ms  p95 XXX.X ms
```

**Copie le bloc entier.** Les quatre étapes comptent autant que le total : c'est
`painted` qui portera le correctif, et `shown` qui dira si l'inversion coûte
quelque chose.

## Étape 3 — Regarder les deux clignotements sur le code d'AUJOURD'HUI

`main` a changé depuis ce matin. Ils sont peut-être différents.

**Ne ferme pas l'application.**

1. `Ctrl` + `Maj` + `2`, puis **Échap**. Est-ce que l'écran « s'allume deux
   fois » comme ce matin, ou est-ce que ça a changé ?
2. `Ctrl` + `Maj` + `2`, puis **un simple clic sans glisser**, puis **Échap**.
   Est-ce que ça fait toujours « comme une photo » ?
   *Je n'écris pas ce que le terminal devrait dire, et c'est délibéré : ta
   première version de ce document annonçait `veil: dismissed`, ce qui était
   vrai sur le `main` de ce matin et ne l'est plus. Depuis les poignées, un clic
   sous le seuil ne ferme plus le voile — il faut Échap. **Dis-moi ce que tu
   lis**, plutôt que de chercher ce que j'aurais prédit.*
3. **Nouveau, à cause des correctifs presse-papier** : `Ctrl` + `Maj` + `2`,
   trace une vraie sélection, `Entrée`. Le voile doit maintenant se fermer
   **après** la copie, pas avant. Est-ce que la fermeture te paraît plus tardive
   qu'avant ? Elle devrait coûter ~10 ms, donc être invisible — si tu la
   remarques, c'est une information.

## Pour finir

`Ctrl` + `C` dans le terminal, puis :

```powershell
Remove-Item Env:\CLICHE_BENCH, Env:\CLICHE_TRANSPORT
```

Sans ça, le prochain `pnpm tauri dev` **dans ce même terminal** relancerait 20
clignotements.

---

## Ce que je veux que tu me rendes

1. Le **bloc de mesure entier** de l'étape 2.
2. Les **trois observations** de l'étape 3.

Avec ça, le correctif que tu as choisi — *n'afficher le voile qu'une fois
l'image décodée* — devient mesurable : on saura ce qu'il coûte, au lieu de
l'espérer.

---

## Ce que ce correctif changera, et ce qu'il faudra surveiller

L'ordre actuel est : `show()` **puis** on dit à la page de charger l'image
(`veil.rs`, lignes 387 et 407), et la peinture arrive ~100 ms plus tard. Pendant
ces ~100 ms, **la fenêtre est visible en affichant ce que la page avait avant** —
l'écran précédent, ou rien au tout premier déclenchement. C'est la cause unique
des deux clignotements, vérifiée dans le code.

Le correctif inverse : la page décode, accuse réception, **puis** Rust montre la
fenêtre. Le voile apparaît complet.

**⚠️ Le piège de mesure, et il vaut d'être lu avant d'écrire une ligne de code.**

*Ce paragraphe corrige ce que ce document affirmait d'abord — que `shown`
grossirait et que `TOTAL` resterait comparable. C'est faux, et le robot de revue
de la PR #7 l'a montré.*

Aujourd'hui, `shown` est marqué **juste après `window.show()`**, et `painted`
seulement à l'accusé de la page. Le décodage tombe donc **entre les deux**, donc
dans `TOTAL`.

Si on décode **avant** `show()` sans rien changer d'autre, cette attente passe
**avant les deux marques**. Elle sort du total. Le correctif aurait alors l'air
**gratuit** — et le chiffre serait plus flatteur qu'avant, non parce qu'on aurait
gagné du temps, mais parce qu'on aurait cessé de le compter.

**Donc le correctif n'est pas seulement un changement d'ordre : les frontières
de mesure doivent bouger avec lui**, pour que l'attente du décodage reste dans un
intervalle rapporté. À trancher au moment de l'écrire, pas à supposer maintenant.
Sans ça, la comparaison avant/après ne veut rien dire.

**Et la sensation, qu'aucune mesure ne tranchera.** Aujourd'hui tu vois quelque
chose (de périmé) à ~25 ms ; après, tu ne verras rien jusqu'à ~125 ms puis le
voile complet. **Le chiffre peut être identique et la sensation plus lente.**
C'est toi qui décideras.
