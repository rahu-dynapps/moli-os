# Adaptateur Intermarché (`intermarche`)

> **Statut : brouillon, pas encore vérifié sur un magasin réglé.** Observé sans compte connecté ni magasin choisi : le
> site demande alors une adresse avant même d'afficher une recherche, et le choix du magasin revient
> à l'humain. Les sections marquées « à observer » restent à remplir dans une session connectée,
> magasin choisi, puis la vérification (3 prix, ajouter / retirer 1 article) à faire.

- Site : `https://www.intermarche.com/`
- Genre : drive (retrait au magasin choisi).

## Le magasin

- **Sans magasin choisi**, l'en-tête montre « Aide et contact », « Carte de fidélité », « Courses en
  ligne » et « Me connecter », et l'accueil est la page générale de l'enseigne (prospectus,
  promotions, « Trouver un magasin »). « Courses en ligne » ou une recherche ouvrent une fenêtre
  « Saisissez votre adresse » avec une carte : **c'est le choix du magasin, réservé à l'humain**.
  L'agent ferme cette fenêtre (Échap), le dit et ne relève rien chez Intermarché.
- Avec un magasin choisi : *à observer* (où l'en-tête affiche le magasin et le mode drive /
  retrait). Le comparer à `settings.store` des réglages de Moli.
- « Me connecter » dans l'en-tête = pas de session : s'arrêter, demander à l'humain de se connecter.

## Chercher

- URL de recherche : `https://www.intermarche.com/recherche/<termes>` (termes encodés, `%20` pour
  l'espace). Sans magasin choisi elle n'affiche rien d'autre que la fenêtre d'adresse.
- Résultats avec magasin choisi : *à observer*.

## La fiche produit

- URL : `https://www.intermarche.com/produit/<slug>/<EAN>`. L'EAN en fin d'URL identifie le produit :
  c'est la `reference` (et l'`ean`) de la fiche dans Moli.
- Prix, prix au kilo / litre, promo, rupture : *à observer* avec `get_page_text`.
- Les produits « en vrac » (fruits, légumes, pain cuit sur place) ont un prix estimé à la pièce : le
  signaler, le poids réel n'est connu qu'au retrait.
- Les promos se présentent en « 2+1 offert », « -50 % sur le 2e », avantage carte… : les noter dans
  `promo`, ne pas changer la quantité pour les déclencher, les proposer en écart.
- Le compte a une page « Mes produits habituels »
  (`/gestion-de-compte/mes-produits-habituels`) dont les liens mènent aux fiches : utile pour
  retrouver la fiche d'un produit déjà acheté.

## Le panier

- Ajouter une quantité : *à observer*.
- Vider le panier : *à observer*.
- URL du panier : *à observer*.

## La limite à ne pas franchir

*À observer* : le bouton du panier qui mène au choix du créneau et au paiement. L'agent s'arrête au
panier rempli, avant ce bouton.

## Pièges vus en vrai

- Bandeau de cookies à la première visite : « Continuer sans accepter » (en haut à droite de la
  fenêtre) le ferme sans consentement.
- `get_page_text` sur l'accueil ne rend que le contenu principal : l'en-tête (compte, magasin,
  panier) se lit avec `find` ou `read_page`.
- Rien ne s'affiche sans magasin choisi : le vérifier en premier.
