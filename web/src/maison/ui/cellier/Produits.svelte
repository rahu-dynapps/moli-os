<script>
  import Icon from '../Icon.svelte';
  import { cellier, saveProduct, confirmOffer, providerName } from '../../lib/cellier.svelte.js';
  import { note } from '../../lib/home.svelte.js';
  import { t } from '../../../lib/i18n.svelte.js';

  /** The products the household buys again and again: how strict a swap may
   *  be, a store they always come from, and their sheet in each chain. */

  const FLEX = ['strict', 'format', 'brand', 'cheapest'];

  const sheets = $derived.by(() => {
    const by = new Map();
    for (const o of cellier.offers) by.set(o.product, [...(by.get(o.product) ?? []), o]);
    return by;
  });

  async function act(run, done) {
    try {
      await run();
      note(done);
    } catch (err) {
      note(err.message, 'error');
    }
  }

  const change = (p, field, value) => act(() => saveProduct({ ...p, [field]: value }), t('cellier.produits.enregistre', { nom: p.name }));
  const offerName = (o) => o.label || o.reference;
</script>

{#if !cellier.products.length}
  <section class="card"><p class="muted">{t('cellier.produits.aucun')}</p></section>
{:else}
  <div class="products">
    {#each cellier.products as p (p.id)}
      <article class="card">
        <h3>{p.name}</h3>
        {#if p.detail}<p class="muted">{p.detail}</p>{/if}

        <div class="fields">
          <label>
            <span>{t('cellier.produits.flexibilite')}</span>
            <select value={p.flexibility} onchange={(e) => change(p, 'flexibility', e.currentTarget.value)}>
              {#each FLEX as f (f)}<option value={f}>{t(`cellier.produits.flex.${f}`)}</option>{/each}
            </select>
          </label>
          <label>
            <span>{t('cellier.produits.enseigne_imposee')}</span>
            <select value={p.fixed_provider ?? ''} onchange={(e) => change(p, 'fixed_provider', e.currentTarget.value || null)}>
              <option value="">{t('cellier.produits.aucune_enseigne')}</option>
              {#each cellier.providers as pr (pr.info.id)}<option value={pr.info.id}>{pr.info.name}</option>{/each}
            </select>
          </label>
        </div>

        <h4>{t('cellier.produits.fiches')}</h4>
        {#if sheets.get(p.id)?.length}
          <ul class="sheets">
            {#each sheets.get(p.id) as o (o.id)}
              <li>
                <div>
                  <b>{providerName(o.provider)}</b>
                  <span>{offerName(o)}</span>
                  <small class="muted">
                    {t('cellier.produits.ref', { ref: o.reference })}{#if o.unit_qty} · {o.unit_qty} {o.unit ?? ''}{/if}
                  </small>
                </div>
                {#if o.status === 'confirmed'}
                  <span class="chip good"><Icon name="ok" size={14} />{t('cellier.produits.confirmee')}</span>
                {:else}
                  <span class="pending">
                    <span class="chip warm">{t('cellier.panier.a_confirmer')}</span>
                    <button
                      onclick={() => act(() => confirmOffer(o.id), t('cellier.produits.confirmee_note'))}
                      aria-label={t('cellier.produits.confirmer_label', { libelle: offerName(o), enseigne: providerName(o.provider) })}
                    >
                      {t('cellier.produits.confirmer')}
                    </button>
                  </span>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted small">{t('cellier.produits.aucune_fiche')}</p>
        {/if}
      </article>
    {/each}
  </div>
{/if}

<style>
  .products {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 380px), 1fr));
    gap: 14px;
  }

  h3 {
    font-size: 17px;
    font-weight: 650;
    overflow-wrap: anywhere;
  }

  h4 {
    margin: 18px 0 6px;
    font-size: 13px;
    font-weight: 650;
    color: var(--ink-3);
  }

  .small {
    font-size: 13px;
  }

  .fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 10px;
    margin-top: 14px;
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 13px;
    font-weight: 600;
    color: var(--ink-2);
  }

  select {
    min-height: 44px;
    min-width: 0;
    padding: 0 12px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--surface-2);
    color: var(--ink);
    font: inherit;
    font-size: 15px;
  }

  .sheets {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .sheets li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 0;
  }

  .sheets li + li {
    border-top: 1px solid var(--line);
  }

  .sheets div {
    display: grid;
    gap: 1px;
    min-width: 0;
  }

  .sheets b {
    font-size: 13px;
    font-weight: 650;
    color: var(--ink-2);
  }

  .sheets span,
  .sheets small {
    overflow-wrap: anywhere;
  }

  .pending {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 6px;
  }

  .pending button {
    min-height: 40px;
    padding: 0 14px;
    border: 0;
    border-radius: 999px;
    background: var(--ink);
    color: var(--surface);
    font-weight: 650;
    font-size: 14px;
  }
</style>
