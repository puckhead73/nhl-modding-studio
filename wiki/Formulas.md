# Formulas

**Historical › Formulas** defines how the historical database's 0-to-20 attributes become EA ratings. A **formula profile** holds, for forwards, defencemen and goalies, a weighted list of inputs per EA attribute. The active profile drives the Attributes tab of every historical player, the estimated overalls in the Roster Builder, and the Legends rankings. The method is described in [Historical Corpus and Rating Formulas](Historical-Corpus-and-Rating-Formulas).

## The editor

- **Normalise**: **Nominal (÷20)** divides every input by the fixed cap of 20; **Position max** divides by the highest value seen in the corpus for that position and attribute, so the all-time best maps to 99.
- Position tabs **Forward**, **Defenseman**, **Goalie**; one row per EA attribute.
- Each row holds input chips with a weight (step 0.1) and ✕ to remove. Drag an input from the left palette (**Custom formulas**, the prebuilt statistical inputs; **FHM attributes**, grouped source attributes) onto a row, or use the row's **+** menu. New inputs start at weight 1.0.
- Every edit auto-saves to the working formula shortly after you stop typing ("Auto-saved.") and takes effect everywhere at once.

## Profiles

The **Profile** dropdown lists **Default (built-in)** and your saved profiles; choosing one loads it and makes it active. Type a name and press **Save** to store the current formula under that name and activate it; **Delete** (shown when the typed name matches a saved profile) removes it immediately, without confirmation.

The Legends status beside the profile reads "Legends: N rated", "Legends: stale — recalculate" or "Legends: not calculated". After a save, an amber strip asks "Calculate Legends ratings for <name> now? It reads every player's every season, so it takes a while…" with **Calculate now** or **Later**. Choosing **Later** leaves an app-wide reminder banner until you calculate.

## Live preview

Search for a player (two or more characters) to add a card; add several to compare. Each card has a dropdown of **Legends** (the composite of each attribute's three best seasons, the rating Puck's Picks ranks on) and every season, and shows the full computed ratings. **Clear all** removes the cards.

Last reviewed: 2026-10-20
