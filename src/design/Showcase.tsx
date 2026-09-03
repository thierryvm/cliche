/*
 * The system, on one page. Reachable at #/systeme.
 *
 * PURELY VISUAL, ON PURPOSE. No `invoke`, no import of ../displays: this page
 * has to open in a plain browser tab, outside the Tauri webview, or it cannot
 * be LOOKED at while it is being written. Every state below is drawn, never
 * fetched.
 *
 * The hover and pressed specimens carry `is-hover` / `is-active`, and the
 * focus specimen carries `is-focus`. Those classes are not a second style
 * sheet: they sit inside the SAME rules as :hover, :active and :focus-visible
 * in components.css, so a specimen cannot drift from the state it claims to
 * show. Disabled uses the real `disabled` attribute, not a class.
 */

import type { ReactNode } from 'react';

import { Glyph, ICON } from './Glyph';
import './components.css';

type SpecimenProps = {
  readonly caption: string;
  readonly wide?: boolean;
  readonly children: ReactNode;
};

function Specimen({ caption, wide = false, children }: SpecimenProps) {
  return (
    <div className={wide ? 'c-specimen c-specimen--wide' : 'c-specimen'}>
      {children}
      <p className="c-caption">{caption}</p>
    </div>
  );
}

type SectionProps = {
  readonly id: string;
  readonly label: string;
  readonly children: ReactNode;
};

function Section({ id, label, children }: SectionProps) {
  return (
    <section className="c-section" aria-labelledby={id}>
      <h2 className="c-section__label" id={id}>
        {label}
      </h2>
      {children}
    </section>
  );
}

const BUTTON_VARIANTS = [
  ['primary', 'Capturer', '--accent / --text-on-solid'],
  ['secondary', 'Ouvrir', '--surface / --border-strong'],
  ['ghost', 'Annuler', 'transparent / --text'],
  ['danger', 'Supprimer', '--danger, rempli au survol'],
] as const;

const BUTTON_STATES = [
  ['', 'repos'],
  ['is-hover', 'survol'],
  ['is-active', 'actif'],
  ['is-focus', 'focus clavier'],
] as const;

export default function Showcase() {
  return (
    <main className="c-showcase">
      <div className="c-showcase__inner">
        <header>
          <h1 className="c-showcase__title">Cliché — le système</h1>
          <p className="c-showcase__lede">
            Chaque composant, dans chaque état, avec le jeton qu&apos;il dépense.
            Aucune valeur n&apos;est écrite ici : tout vient de{' '}
            <code>src/design/tokens.css</code>.
          </p>
        </header>

        {/* ---------------------------------------------------------- */}
        <Section id="s-type" label="Typographie — quatre tailles, rapport 1,25">
          <div className="c-panel c-stack">
            <Specimen caption="--size-title · --font-display · une par fenêtre" wide>
              <p
                className="c-scale__sample"
                style={{ fontSize: 'var(--size-title)', fontFamily: 'var(--font-display)' }}
              >
                Cliché
              </p>
            </Specimen>
            <Specimen caption="--size-section · un titre de section, ou un chiffre montré" wide>
              <p className="c-scale__sample" style={{ fontSize: 'var(--size-section)' }}>
                Capture 3 sur 12
              </p>
            </Specimen>
            <Specimen caption="--size-body · tout ce qu'on lit pour agir" wide>
              <p className="c-scale__sample" style={{ fontSize: 'var(--size-body)' }}>
                Enregistrer la sélection dans la bibliothèque
              </p>
            </Specimen>
            <Specimen caption="--size-caption · --font-numeric · faits en pixels, unités, horodatages" wide>
              <p
                className="c-scale__sample"
                style={{
                  fontSize: 'var(--size-caption)',
                  fontFamily: 'var(--font-numeric)',
                  color: 'var(--text-muted)',
                }}
              >
                1920×1080 px · échelle 1.5 · 14:32:07
              </p>
            </Specimen>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-buttons" label="Boutons — quatre variantes, cinq états">
          <div className="c-panel c-stack">
            {BUTTON_VARIANTS.map(([variant, label, note]) => (
              <div key={variant}>
                <p className="c-caption">
                  .c-btn--{variant} — {note}
                </p>
                <div className="c-specimens" style={{ marginBlockStart: 'var(--space-3)' }}>
                  {BUTTON_STATES.map(([stateClass, stateLabel]) => (
                    <Specimen key={stateLabel} caption={stateLabel}>
                      <button
                        type="button"
                        className={`c-btn c-btn--${variant}${stateClass ? ` ${stateClass}` : ''}`}
                      >
                        {label}
                      </button>
                    </Specimen>
                  ))}
                  <Specimen caption="désactivé · bord tireté (A4)">
                    <button type="button" className={`c-btn c-btn--${variant}`} disabled>
                      {label}
                    </button>
                  </Specimen>
                </div>
              </div>
            ))}

            <p className="c-caption">
              Le focus est dessiné avec --focus-ring-offset : il atterrit sur la
              surface, jamais sur l&apos;accent. Tabulez dans la page pour le voir
              en vrai — la classe de démonstration peint la même règle.
            </p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-icons" label="Bouton icône — 44 px, c'est une RULE">
          <div className="c-panel c-stack">
            <div className="c-specimens">
              <Specimen caption="repos · --hit-min">
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Capturer">
                  <Glyph d={ICON.capture} />
                </button>
              </Specimen>
              <Specimen caption="survol">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon is-hover"
                  aria-label="Annoter"
                >
                  <Glyph d={ICON.pen} />
                </button>
              </Specimen>
              <Specimen caption="focus clavier">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon is-focus"
                  aria-label="Masquer"
                >
                  <Glyph d={ICON.mask} />
                </button>
              </Specimen>
              <Specimen caption="outil actif · aria-pressed + barre">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon"
                  aria-pressed="true"
                  aria-label="Copier"
                >
                  <Glyph d={ICON.copy} />
                </button>
              </Specimen>
              <Specimen caption="désactivé · --surface-inert">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon"
                  aria-label="Supprimer"
                  disabled
                >
                  <Glyph d={ICON.trash} />
                </button>
              </Specimen>
            </div>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-fields" label="Champs de saisie">
          <div className="c-panel c-stack">
            <div className="c-specimens">
              <Specimen caption="repos · --border-strong (jamais --border)">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-rest">
                    Nom du fichier
                  </label>
                  <input
                    id="f-rest"
                    className="c-input"
                    type="text"
                    defaultValue="capture-2026-09-03"
                  />
                </div>
              </Specimen>

              <Specimen caption="focus · bord --accent + anneau">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-focus">
                    Dossier
                  </label>
                  <input id="f-focus" className="c-input is-focus" type="text" defaultValue="Captures" />
                </div>
              </Specimen>

              <Specimen caption="désactivé · bord tireté + --text-inert">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-off">
                    Rétention
                  </label>
                  <input id="f-off" className="c-input" type="text" defaultValue="30 jours" disabled />
                  <span className="c-field__message">Réglable quand la bibliothèque est active.</span>
                </div>
              </Specimen>

              <Specimen caption="erreur · aria-invalid + message nommé">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-err">
                    Raccourci
                  </label>
                  <input
                    id="f-err"
                    className="c-input"
                    type="text"
                    defaultValue="Ctrl+Maj+A"
                    aria-invalid="true"
                    aria-describedby="f-err-msg"
                  />
                  <span className="c-field__message c-field__message--error" id="f-err-msg">
                    <Glyph d={ICON.alert} />
                    Raccourci déjà pris par Windows.
                  </span>
                </div>
              </Specimen>

              <Specimen caption="numérique · --font-numeric, chiffres alignés">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-num">
                    Largeur
                  </label>
                  <input
                    id="f-num"
                    className="c-input c-input--numeric"
                    type="text"
                    defaultValue="1920"
                  />
                </div>
              </Specimen>

              <Specimen caption="vide · le premier jour, --text-muted">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-empty">
                    Préfixe
                  </label>
                  <input id="f-empty" className="c-input" type="text" placeholder="cliché-" />
                </div>
              </Specimen>
            </div>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-glass" label="Verre — sur les deux fonds extrêmes">
          <div className="c-band">
            <div className="c-band__half c-band__half--dark" />
            <div className="c-band__half c-band__half--light" />
            <div className="c-band__pinstripe" />
            <div className="c-glass">
              <h3 className="c-glass__title">Sélection</h3>
              <p className="c-glass__facts">1204×780 px · échelle 1.5 · PNG</p>
              <div
                className="c-toolbar"
                style={{ marginBlockStart: 'var(--space-3)', padding: 0 }}
                role="toolbar"
                aria-label="Outils de capture"
              >
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Recadrer">
                  <Glyph d={ICON.capture} />
                </button>
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon"
                  aria-pressed="true"
                  aria-label="Annoter"
                >
                  <Glyph d={ICON.pen} />
                </button>
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Masquer">
                  <Glyph d={ICON.mask} />
                </button>
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Copier">
                  <Glyph d={ICON.copy} />
                </button>
                <button type="button" className="c-btn c-btn--primary" style={{ minWidth: 'var(--hit-min)' }}>
                  Copier
                </button>
              </div>
            </div>
            <p className="c-band__caption">moitié noire · moitié blanche</p>
          </div>

          <ul className="c-rules" style={{ marginBlockStart: 'var(--space-4)' }}>
            <li>
              Le cas qui contraint est le fond <strong>blanc en thème sombre</strong> :
              le plancher d&apos;alpha y est 0,787 pour 0,80 déclaré, soit 0,013 de
              marge. La valeur vient de <code>--glass-alpha</code> dans{' '}
              <code>tokens.css</code>, et <code>scripts/check-contrast.mjs</code> la
              recalcule à chaque <code>pnpm test</code> et en CI : baisser l&apos;alpha
              sous ce plancher fait échouer la vérification, pas seulement avertir.
            </li>
            <li>
              <code>--text-inert</code> est interdit sur verre (RULE) : aucun contrôle
              désactivé n&apos;est posé dans ce panneau.
            </li>
            <li>
              Un seul panneau de verre sur cette page. La RULE en autorise trois par
              écran, jamais imbriqués, jamais dans une liste.
            </li>
          </ul>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-rows" label="Lignes de liste">
          <div className="c-panel">
            {/* role=listbox/option, not plain buttons: aria-selected only means
                something inside a selection widget, and the CSS keys on it. */}
            <ul className="c-list" role="listbox" aria-label="Captures récentes">
              <li role="presentation">
                <button type="button" role="option" aria-selected="false" className="c-row">
                  <Glyph d={ICON.copy} />
                  <span className="c-row__name">capture-2026-09-03-143207.png</span>
                  <span className="c-row__facts">1920×1080</span>
                </button>
              </li>
              <li role="presentation">
                <button type="button" role="option" aria-selected="false" className="c-row is-hover">
                  <Glyph d={ICON.copy} />
                  <span className="c-row__name">écran-de-connexion-client.png</span>
                  <span className="c-row__facts">1204×780</span>
                </button>
              </li>
              <li role="presentation">
                <button type="button" role="option" className="c-row" aria-selected="true">
                  <Glyph d={ICON.check} />
                  <span className="c-row__name">rapport-trimestriel-page-4.png</span>
                  <span className="c-row__facts">2560×1440</span>
                </button>
              </li>
              <li role="presentation">
                <button type="button" role="option" aria-selected="false" className="c-row is-focus">
                  <Glyph d={ICON.copy} />
                  <span className="c-row__name">
                    un-nom-de-fichier-beaucoup-trop-long-pour-la-fenêtre-de-480-pixels.png
                  </span>
                  <span className="c-row__facts">800×600</span>
                </button>
              </li>
            </ul>
          </div>
          <p className="c-caption" style={{ marginBlockStart: 'var(--space-3)' }}>
            séparateur --border (décoratif) · sélection --surface-selected + barre
            --accent + coche · débordement coupé à l&apos;ellipse
          </p>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-grid" label="Bibliothèque — 2 colonnes à 480 px (2×212 + 24 = 448)">
          <ul className="c-grid" role="listbox" aria-label="Bibliothèque">
            <li role="presentation">
              <button type="button" role="option" aria-selected="false" className="c-tile">
                <span className="c-tile__frame" />
                <span className="c-tile__name">capture-2026-09-03.png</span>
                <span className="c-tile__facts">1920×1080 · 412 Ko</span>
              </button>
            </li>
            <li role="presentation">
              <button type="button" role="option" className="c-tile" aria-selected="true">
                <span className="c-tile__frame">
                  <span className="c-tile__badge">
                    <Glyph d={ICON.check} />
                  </span>
                </span>
                <span className="c-tile__name">écran-de-connexion.png</span>
                <span className="c-tile__facts">1204×780 · 208 Ko</span>
              </button>
            </li>
            <li role="presentation">
              <button type="button" role="option" aria-selected="false" className="c-tile is-focus">
                <span className="c-tile__frame" />
                <span className="c-tile__name">
                  un-nom-de-fichier-beaucoup-trop-long-pour-tenir.png
                </span>
                <span className="c-tile__facts">2560×1440 · 1,2 Mo</span>
              </button>
            </li>
            <li role="presentation">
              {/* The placeholder wears the SAME classes as the tile it stands
                  in for — __frame, __name, __facts — so it reserves the exact
                  box the loaded tile will take, by construction and not by a
                  re-typed height. It used to skip the __name line, and the
                  whole row jumped up when a thumbnail arrived. The word stays
                  in the markup for a screen reader; .c-skeleton paints it out. */}
              <span className="c-tile" aria-busy="true">
                <span className="c-tile__frame c-skeleton" />
                <span className="c-tile__name c-skeleton" aria-hidden="true">
                  &nbsp;
                </span>
                <span className="c-tile__facts c-skeleton">chargement…</span>
              </span>
            </li>
          </ul>

          <div className="c-stack" style={{ marginBlockStart: 'var(--space-5)' }}>
            <div className="c-empty">
              <Glyph d={ICON.capture} />
              <h3 className="c-empty__title">Aucune capture</h3>
              <p style={{ margin: 0 }}>
                Ctrl + Maj + 2 découpe une zone. Rien ne quitte cette machine.
              </p>
              <button type="button" className="c-btn c-btn--primary">
                Capturer une zone
              </button>
            </div>
            <p className="c-caption">état vide · --space-7, le seul endroit qui le dépense</p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-notes" label="Messages — l'emploi, jamais l'émotion">
          <div className="c-stack">
            <div className="c-note c-note--danger" role="alert">
              <Glyph d={ICON.alert} />
              <span>
                <strong>Échec</strong> — le fichier n&apos;a pas pu être écrit :
                dossier en lecture seule.
              </span>
            </div>
            <div className="c-note c-note--warning">
              <Glyph d={ICON.info} />
              <span>
                <strong>Irréversible</strong> — appliquer le masque grave les pixels
                dans le fichier livré.
              </span>
            </div>
            <div className="c-note c-note--success" role="status">
              <Glyph d={ICON.check} />
              <span>
                <strong>Copié</strong> — l&apos;image est dans le presse-papiers.
              </span>
            </div>
            <p className="c-caption">
              --danger / --warning / --success, chacun doublé d&apos;un mot et d&apos;une
              icône (PRD A4)
            </p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-surfaces" label="Surfaces et lignes">
          <div className="c-panel">
            <div className="c-swatches">
              {[
                ['--bg-window', 'fenêtre, sans Mica'],
                ['--surface', 'le plan par défaut'],
                ['--surface-raised', 'menus, popovers'],
                ['--surface-inert', 'contrôle inutilisable'],
                ['--surface-selected', 'la sélection'],
                ['--accent', 'action, actif, focus'],
                ['--border-strong', 'bord de contrôle'],
                ['--border', 'séparateur décoratif'],
              ].map(([token, use]) => (
                <div className="c-swatch" key={token}>
                  <span className="c-swatch__chip" style={{ backgroundColor: `var(${token})` }} />
                  <span className="c-caption">{token}</span>
                  <span className="c-caption">{use}</span>
                </div>
              ))}
            </div>
          </div>
        </Section>

        <Section id="s-rules" label="Ce que ce matériau s'interdit">
          <ul className="c-rules">
            <li>
              <code>--border</code> ne borde jamais un contrôle — uniquement deux
              lignes de même nature. Il mesure 1,33:1 sur <code>--surface</code>, et
              de 1,01:1 à 1,52:1 sur les cinq surfaces des deux thèmes : loin des
              3:1 qu&apos;exige WCAG 1.4.11 pour une limite de contrôle. Chiffres
              recalculés depuis <code>tokens.css</code> par{' '}
              <code>scripts/check-contrast.mjs</code>, 139 paires à chaque{' '}
              <code>pnpm test</code>.
            </li>
            <li>
              <code>--text-inert</code> ne va jamais sur le verre.
            </li>
            <li>
              Un bouton ne porte que <code>--elev-0</code> ou <code>--elev-1</code> :
              au-delà, c&apos;est un plan surélevé, pas un contrôle.
            </li>
            <li>
              L&apos;accent ne sert qu&apos;à quatre choses : action principale, état
              actif, anneau de focus, poignées de sélection.
            </li>
            <li>
              Aucun <code>@media</code> dans <code>components.css</code> : thème sombre
              et replis du verre sont déjà dans les jetons.
            </li>
          </ul>
        </Section>
      </div>
    </main>
  );
}
