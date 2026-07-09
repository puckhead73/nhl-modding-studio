# FHM source-data name corruption — unrecovered entries

190 corpus persons carry a literal `?` in their name where the FHM source data dropped a diacritic (verified: identical corruption in the CSV export, the prior app's `historical.db`, and its `player_jersey_numbers`/`team_captaincy` tables — the corruption originates at a single point, the newest 2025-2026 season generation, with no clean copy anywhere we have access to). One case (`chmelar_jaroslav_20_07_2003`) WAS recoverable via 5 earlier clean seasons and has been fixed (see `crates/tdb-fhm/src/fixups.rs`). The rest are listed below for reference; do not guess-restore their spelling without a verified source — add a new `fixups.rs` entry if one is ever found.

| person_id | Name | Nationality | DOB | Seasons |
|---|---|---|---|---|
| 96965 | Frans ?berg | Sweden | 2010-03-11 | 2026-2026 |
| 96195 | Ted ?berg | Sweden | 2009-03-04 | 2026-2026 |
| 96804 | Joel ?hgren | Sweden | 2009-12-04 | 2026-2026 |
| 96661 | Oliver ?hlin | Sweden | 2009-05-24 | 2026-2026 |
| 96540 | Max ?jercrantz | Sweden | 2008-02-07 | 2026-2026 |
| 96336 | Elias ?kerberg | Sweden | 2010-04-01 | 2026-2026 |
| 97173 | Celwin ?kerlind | Sweden | 2011-08-13 | 2026-2026 |
| 96980 | Ferhat Efe ?nsever | Türkiye | 2010-07-05 | 2026-2026 |
| 96930 | Love ?stberg | Sweden | 2009-03-05 | 2026-2026 |
| 96132 | Vilmer ?stberg | Sweden | 2008-07-22 | 2026-2026 |
| 97042 | Mads ?stergaard | Denmark | 2004-01-01 | 2026-2026 |
| 97038 | Math?o Ahullo | France | 2010-10-14 | 2026-2026 |
| 97264 | Edvard Alml?w | Sweden | 2008-02-28 | 2026-2026 |
| 96432 | Jan B?hler | Germany | 2009-09-14 | 2026-2026 |
| 95784 | Lyon B?rngen | Germany | 2008-12-20 | 2026-2026 |
| 97140 | Mat?j Bany?k | Czechia | 2011-05-25 | 2026-2026 |
| 86246 | Alex Bartovi? | Czechia | 2010-08-07 | 2026-2026 |
| 97067 | Patrik Be?va??k | Czechia | 2011-06-06 | 2026-2026 |
| 96035 | Casper Bergstr?m | Sweden | 2009-03-23 | 2026-2026 |
| 96148 | Jack Bj?rk | Sweden | 2009-04-29 | 2026-2026 |
| 96527 | Pelle Bj?rk | Sweden | 2009-07-31 | 2026-2026 |
| 96826 | Carl-Adrian Bj?rn | Sweden | 2010-04-01 | 2026-2026 |
| 96784 | Oskar Blomb?ck | Sweden | 2009-08-25 | 2026-2026 |
| 95906 | Alexander Borgstr?m | Sweden | 2008-09-07 | 2026-2026 |
| 97207 | Joni Bostr?m | Finland | 2007-01-01 | 2026-2026 |
| 96813 | Gil Brechb?hl | Switzerland | 2008-04-22 | 2026-2026 |
| 96941 | Henry Brem?s | Sweden | 2009-08-31 | 2026-2026 |
| 95972 | Tim Bro?-Pawlak | Germany | 2007-07-28 | 2026-2026 |
| 96949 | Aleksander T?rmo Bryni | Norway | 2009-01-01 | 2026-2026 |
| 95757 | Jakub Buben?k | Czechia | 2003-10-25 | 2026-2026 |
| 96707 | Bj?rn Carlsson | Sweden | 2009-02-28 | 2026-2026 |
| 96555 | Milo Carlsw?rd | Sweden | 2009-05-29 | 2026-2026 |
| 96259 | No? Clavel | France | 2010-03-14 | 2026-2026 |
| 95962 | Leon D?nkel | Germany | 2008-02-12 | 2026-2026 |
| 96589 | Charlie Dafg?rd Larsson | Sweden | 2010-03-22 | 2026-2026 |
| 96747 | L?o Dal Pan | France | 2009-01-01 | 2026-2026 |
| 96316 | Pelle Danerl?v | Sweden | 2008-10-16 | 2026-2026 |
| 96276 | Yoann Patrice Doum?ne | Switzerland | 2010-01-01 | 2026-2026 |
| 96359 | Simon Ekl?f | Sweden | 2009-02-15 | 2026-2026 |
| 96893 | Rapha?l Eliard | France | 2009-04-10 | 2026-2026 |
| 96475 | Marius Est?en | Sweden | 2009-05-25 | 2026-2026 |
| 96123 | Elias F?lt | Sweden | 2009-01-09 | 2026-2026 |
| 96765 | Lars F?ssl | Brazil | 2006-07-17 | 2026-2026 |
| 96218 | Timoth?e Fengos | Canada | 2008-02-27 | 2026-2026 |
| 96030 | Eric Fj?llman | Sweden | 2009-09-23 | 2026-2026 |
| 96967 | Theodor Fl?m | Norway | 2009-08-19 | 2026-2026 |
| 96660 | John-Diego Flisb?ck | Sweden | 2009-07-20 | 2026-2026 |
| 97050 | Mat?j Fr?ba | Czechia | 2011-02-17 | 2026-2026 |
| 95773 | Tim Fr?hlichs | Netherlands | 2007-11-10 | 2026-2026 |
| 96806 | Felix Frojd?n | Sweden | 2010-01-27 | 2026-2026 |
| 96377 | Jonathan G?llstr?m | Sweden | 2009-11-01 | 2026-2026 |
| 96796 | Axel G?ransson | Sweden | 2009-09-20 | 2026-2026 |
| 96866 | Aim?e Georgi | Germany | 2010-10-15 | 2026-2026 |
| 96118 | Alfred Goden?s | Sweden | 2008-09-30 | 2026-2026 |
| 96622 | Xaver Gr?ner | Germany | 2009-12-02 | 2026-2026 |
| 96178 | Mika Gra? | Germany | 2011-07-08 | 2026-2026 |
| 95868 | Simon H?gardh | Sweden | 2007-09-27 | 2026-2026 |
| 96950 | Fabian H?glund | Sweden | 2009-08-20 | 2026-2026 |
| 96098 | Max H?glund | Sweden | 2008-06-27 | 2026-2026 |
| 96523 | Louie H?kansson | Sweden | 2009-04-23 | 2026-2026 |
| 96236 | Ludvig H?kansson | Sweden | 2009-08-15 | 2026-2026 |
| 96499 | Malte H?ll Lillberg | Sweden | 2009-05-22 | 2026-2026 |
| 97227 | Moris H?rlimann | Switzerland | 2008-03-20 | 2026-2026 |
| 96913 | Felix H?rmann | Germany | 2009-08-29 | 2026-2026 |
| 96329 | Elias Hagstr?m Galjaard | Sweden | 2008-01-16 | 2026-2026 |
| 96474 | Per Hammarb?ck | Sweden | 2009-12-08 | 2026-2026 |
| 97098 | Sascha Heuz? | France | 2010-04-26 | 2026-2026 |
| 95824 | Joakim Hirsti? | Finland | 1998-11-12 | 2026-2026 |
| 96983 | Jan Martin Hlinsk? | Czechia | 2011-04-17 | 2026-2026 |
| 95891 | Erik Holmstr?m | Sweden | 2007-03-06 | 2026-2026 |
| 96417 | Ludvig Hyl?n | Sweden | 2009-02-16 | 2026-2026 |
| 95725 | Hugo Hyv?rinen | Finland | 2004-01-07 | 2026-2026 |
| 95858 | Justin J?rs | Germany | 2009-07-06 | 2026-2026 |
| 96879 | Andreas J?rvinen | Finland | 2010-06-19 | 2026-2026 |
| 96235 | Albin K?ck | Sweden | 2008-03-29 | 2026-2026 |
| 96929 | Malte K?gstr?m | Sweden | 2009-02-27 | 2026-2026 |
| 97116 | Jimi K?hk?nen | Finland | 2010-01-01 | 2026-2026 |
| 97117 | Weeti K?hk?nen | Finland | 2009-01-01 | 2026-2026 |
| 95862 | Alexander K?nig | Germany | 2009-04-06 | 2026-2026 |
| 97124 | Sebastiano K?ppeli | Switzerland | 2010-08-07 | 2026-2026 |
| 96910 | Erik Kaarb? | Norway | 2009-01-01 | 2026-2026 |
| 96759 | Victor Slettebr?ten Karadas | Norway | 2009-01-01 | 2026-2026 |
| 96277 | Zsombor Kov?cs | Hungary | 2008-03-10 | 2026-2026 |
| 96976 | Petr Kova??k | Czechia | 2011-08-24 | 2026-2026 |
| 97024 | Milo Kvarnef?lt | Sweden | 2010-06-26 | 2026-2026 |
| 95833 | Eetu L?f | Finland | 2000-05-23 | 2026-2026 |
| 96818 | Phileas L?nger | Germany | 2011-07-25 | 2026-2026 |
| 95837 | Felix L?nnquist | Sweden | 2006-11-20 | 2026-2026 |
| 96340 | Liam L?vgren Lennartsson | Sweden | 2009-08-04 | 2026-2026 |
| 97030 | Vincent Larsg?rden | Sweden | 2010-05-05 | 2026-2026 |
| 95806 | Philippe Lepor? | Canada | 2006-11-05 | 2026-2026 |
| 96449 | Neo Leskel? | Finland | 2008-06-08 | 2026-2026 |
| 97072 | Melker Lid?n | Sweden | 2010-07-13 | 2026-2026 |
| 95754 | Leonard Lim? | Sweden | 2005-03-04 | 2026-2026 |
| 96379 | Elias Lind?n | Sweden | 2008-12-25 | 2026-2026 |
| 95762 | Vidar Lindstr?m | Sweden | 2006-02-09 | 2026-2026 |
| 97213 | Oliwer Lud?n | Finland | 2008-01-01 | 2026-2026 |
| 97220 | Luka M??tt? | Finland | 2012-01-01 | 2026-2026 |
| 95687 | Jere M?kel? | Finland | 2000-09-19 | 2026-2026 |
| 96998 | Martti M?kel? | Finland | 2005-04-18 | 2026-2026 |
| 96520 | Joel M?kitalo | Sweden | 2010-05-29 | 2026-2026 |
| 96437 | Maximilian M?ller | Germany | 2009-08-29 | 2026-2026 |
| 95677 | Tomas M?ller | Czechia | 2001-04-05 | 2026-2026 |
| 96546 | Viggo M?nsson | Sweden | 2010-07-07 | 2026-2026 |
| 96766 | Martim Mar?al | Portugal | 2008-05-27 | 2026-2026 |
| 96773 | Ville Martinm?ki | Finland | 2009-01-01 | 2026-2026 |
| 95696 | Maisa Mastom?ki | Finland | 2005-03-07 | 2026-2026 |
| 96664 | Vojt?ch Mertl | Czechia | 2010-03-29 | 2026-2026 |
| 95668 | Lari Mykr? | Finland | 1988-05-12 | 2026-2026 |
| 97182 | Samuel Myyryl?inen | Finland | 2010-01-01 | 2026-2026 |
| 96314 | Isac N?sstr?m | Sweden | 2008-05-28 | 2026-2026 |
| 86341 | Ond?ej Nabelek | Czechia | 2010-11-23 | 2026-2026 |
| 96494 | Jonathan Navstr?m | Sweden | 2009-12-12 | 2026-2026 |
| 95748 | Max Niemel? | Finland | 2004-04-27 | 2026-2026 |
| 96492 | Luka Kaatorp Nilsg?rd | Norway | 2009-09-24 | 2026-2026 |
| 97017 | Simon Nord Nor?n | Sweden | 2010-09-21 | 2026-2026 |
| 97045 | Pavel Nov? | Czechia | 2011-03-06 | 2026-2026 |
| 97043 | Vojt?ch Nov??ek | Czechia | 2011-03-26 | 2026-2026 |
| 97053 | Anton?n Novotn? | Czechia | 2011-07-14 | 2026-2026 |
| 96341 | Anton Nyr?n | Sweden | 2009-01-02 | 2026-2026 |
| 96612 | Lukas Ohlson S?derlind | Sweden | 2009-10-17 | 2026-2026 |
| 96694 | Onni Ojanper? | Finland | 2009-02-14 | 2026-2026 |
| 96531 | Lucas Ols?n | Sweden | 2009-03-12 | 2026-2026 |
| 10527 | Theodor Pis?te?k | Czechia | 2001-06-26 | 2026-2026 |
| 96562 | Jan Pl?nka | Czechia | 2010-06-27 | 2026-2026 |
| 97164 | Ond?ej Pol??ek | Czechia | 2011-02-16 | 2026-2026 |
| 44742 | Bálint Pákozdi? | England | 1999-05-03 | 2026-2026 |
| 13969 | Levente Pákozdi? | Hungary | 2004-08-21 | 2026-2026 |
| 97193 | Otto R?is?nen | Finland | 2006-08-15 | 2026-2026 |
| 96586 | Paul Reu? | Germany | 2011-04-10 | 2026-2026 |
| 96750 | Akseli Riihim?ki | Finland | 2010-01-26 | 2026-2026 |
| 96337 | Ernst Ros?n | Sweden | 2009-05-21 | 2026-2026 |
| 96297 | Timoth?e Rossat-Mignot | France | 2010-09-17 | 2026-2026 |
| 97233 | J?r?my Rothpletz | Switzerland | 2008-10-26 | 2026-2026 |
| 95728 | Charles-?tienne Roy | Canada | 2006-10-26 | 2026-2026 |
| 96489 | Adam Rysk?sen | Sweden | 2009-06-30 | 2026-2026 |
| 181048 | Igor S?hadilov | Russia | 1980-06-07 | 1997-2025 |
| 97222 | Leo S?kkinen | Finland | 2008-01-01 | 2026-2026 |
| 97259 | Phillip S?rensen | Denmark | 2004-01-01 | 2026-2026 |
| 96790 | J?rgen Sabo | Slovakia | 2012-06-17 | 2026-2026 |
| 96066 | Vilmer Sandstr?m | Sweden | 2008-10-01 | 2026-2026 |
| 96536 | Maximilian Sch?fer | Germany | 2011-03-02 | 2026-2026 |
| 97082 | Jaroslav Sedl??ek | Czechia | 2011-02-23 | 2026-2026 |
| 96344 | Leon Sennstr?m | Sweden | 2008-07-24 | 2026-2026 |
| 97033 | Perttu Sepp?nen | Finland | 2009-07-14 | 2026-2026 |
| 96014 | Ossi Siljam?ki | Finland | 2007-07-13 | 2026-2026 |
| 96032 | Vincent Sj?? Laisio | Sweden | 2008-10-31 | 2026-2026 |
| 95771 | Vilmer Sj?blom | Sweden | 2006-03-02 | 2026-2026 |
| 96029 | Erik Sj?din | Sweden | 2008-05-16 | 2026-2026 |
| 97153 | Topi Sj?holm | Finland | 2010-08-15 | 2026-2026 |
| 96933 | Fabian Sj?lander | Sweden | 2010-11-03 | 2026-2026 |
| 96945 | Tyler Sj?lin | Sweden | 2009-09-17 | 2026-2026 |
| 97022 | Marius Rotefoss Skr?der | Norway | 2009-01-01 | 2026-2026 |
| 97159 | Hugo Skyttg?rd Holm | Sweden | 2010-08-21 | 2026-2026 |
| 96298 | Viktor Langbr?ten Smestad | Norway | 2009-06-14 | 2026-2026 |
| 96828 | B?lint Solt?zs | Hungary | 2009-02-26 | 2026-2026 |
| 96431 | Joakim St?hl | Sweden | 2009-10-09 | 2026-2026 |
| 95758 | Anton St?lberg | Sweden | 2006-12-06 | 2026-2026 |
| 95759 | Oliver St?lberg | Sweden | 2006-12-06 | 2026-2026 |
| 97176 | Gabriel Str?m | Sweden | 2010-01-20 | 2026-2026 |
| 97037 | Alvin Str?mberg | Sweden | 2010-02-22 | 2026-2026 |
| 96946 | Noah Str?mberg | Sweden | 2012-01-11 | 2026-2026 |
| 96712 | Viggo Sundb?ck | Sweden | 2009-03-24 | 2026-2026 |
| 95693 | Conny Sundstr?m | Finland | 1997-05-22 | 2026-2026 |
| 95689 | Kent Sundstr?m | Finland | 1995-12-30 | 2026-2026 |
| 80858 | Bende Gerg? Szongoth | Hungary | 2008-04-20 | 2026-2026 |
| 96248 | Alvin T?gt | Sweden | 2010-02-23 | 2026-2026 |
| 96070 | Leon T?ngring | Sweden | 2008-12-17 | 2026-2026 |
| 96317 | Oskar T?rm? | Sweden | 2008-11-15 | 2026-2026 |
| 96251 | Levon Tr?sch | Switzerland | 2010-07-17 | 2026-2026 |
| 96286 | H?seyin Kamil Tuna Kaya | Türkiye | 2008-06-24 | 2026-2026 |
| 96748 | Renne V??t?inen | Finland | 2008-09-30 | 2026-2026 |
| 97208 | Miika V?h?s?yrinki | Finland | 1995-01-01 | 2026-2026 |
| 95695 | Jesse V?lj? | Finland | 2001-12-13 | 2026-2026 |
| 95819 | Henry V?nttil? | Finland | 1988-11-06 | 2026-2026 |
| 96012 | J?r?my Vaillancourt | Canada | 2007-07-25 | 2026-2026 |
| 96795 | Olavi Valkeap?? | Finland | 2009-07-27 | 2026-2026 |
| 97216 | V?in? Vanhanen | Finland | 2009-01-01 | 2026-2026 |
| 96768 | David Varg? | Sweden | 2010-03-22 | 2026-2026 |
| 95998 | Bastien W?lti | Switzerland | 2009-05-27 | 2026-2026 |
| 96999 | Theodor W?rman | Sweden | 2011-02-05 | 2026-2026 |
| 96490 | Perrin W?sthed Lindwall | Sweden | 2009-03-07 | 2026-2026 |
| 95975 | Lenny Waa?mann | Germany | 2009-06-12 | 2026-2026 |
| 96026 | Isak Wahnstr?m | Sweden | 2009-04-03 | 2026-2026 |
| 95965 | Christoffer Andr? Wallace | Norway | 2008-01-01 | 2026-2026 |
| 96479 | Jacob Wikh?ll | Sweden | 2009-04-01 | 2026-2026 |
| 97177 | Jyri Ylim?ki | Finland | 2002-01-01 | 2026-2026 |
| 96495 | Emil Ylv?n | Sweden | 2009-06-17 | 2026-2026 |
| 96780 | Yagiz G?kt?rk Yorat | Türkiye | 2009-07-09 | 2026-2026 |
| 97231 | Kian von Niederh?usern | Switzerland | 2008-04-21 | 2026-2026 |
