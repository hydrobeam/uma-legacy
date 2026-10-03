# Conditional character events that give skill hints

Source: GameTora `training_events/char` (the per-character section of events with trigger conditions, which holds the
secret events and the race-result events). Rewards decoded from `dict_evrew` (offset 36), choice rewards only.
`*` = one of the default 8 targets. With several choices, IT takes choice 1 unless a prioritized skill is on another.
Race feasibility is checked against the solver's Global G1 data: "impossible" = never offered in that year, or on a
turn forced to another career goal. Non-G1 races and other condition kinds are shown raw and not checked.
Older versions (GameTora tags such as pre_2nd_anni) appear only where hints or conditions differ.


## Admire Vega

- **A Synergy of Stars**: End Closer Straightaways ○ +2, End Closer Corners ○ +2, End Closer Corners ○ +2, End Closer Corners ○ +1  
  needs: fires: Senior month 12 late; rn_race_w ["T.M. Opera O", "Meisho Doto", "Narita Top Road"] 8

## Agnes Digital

- **After the Mile Ch. Nambu Hai: This Is Heaven**: Morioka Racecourse ○ +1, Rapid Gain +1  
  needs: win M.C. Nambu Hai
- **All-Round Ruler**: Updrafters +1, Unyielding Spirit +1  
  needs: win_or 1 of: Hanshin Juvenile Fillies, Asahi Hai Futurity Stakes; win Oka Sho; win NHK Mile Cup; win_or 1 of: Mile Championship (Classic), Mile Championship (Senior); win February Stakes; win Victoria Mile; win_or 1 of: Yasuda Kinen (Classic), Yasuda Kinen (Senior)

## Agnes Tachyon

- **The Triple Crown Champion That Was**: Tactical Tweak +2  
  needs: triple_crown

## Air Shakur

- **After the Japan C.: Noise**: Fighting Spirit +3  
  needs: win Japan Cup (Classic)
- **After the Kikuka Sho: King of the Game**: Corner Adept ○ +2  
  needs: triple_crown
- **It's a New Day**: Fall Runner ○ +2  
  needs: autumn_triple_crown_senior
- **It's a New Day** (pre_2nd_anni): Fall Runner ○ +1  
  needs: autumn_triple_crown_senior
- **Just as Planned!**: Spring Runner ○ +2  
  needs: spring_triple_crown
- **Just as Planned!** (pre_2nd_anni): Spring Runner ○ +1  
  needs: spring_triple_crown

## Aston Machan

- **After the Shuka Sho: Machan's Here Too**: Rosy Outlook +2  
  needs: win Shuka Sho
- **After the Tenno Sho (Autumn): A Legendary Clash**: Up-Tempo +2  
  needs: win Tenno Sho (Autumn) (Senior)
- **Say It to Machan, Too**: Productive Plan +2  
  needs: win_n_of 3 of: Chukyo Junior Stakes, Shinzan Kinen, Oka Sho, Rose Stakes; fires: Classic month 12 late
- **Tell Machan**: Shifting Gears +2  
  needs: win_n_of 4 of: Hanshin Juvenile Fillies, Elfin Stakes, Oka Sho, Victoria Mile, Yasuda Kinen (Senior), Mainichi Okan (Senior); fires: Senior month 12 late

## Bamboo Memory

- **I'm the BEST OF THE BEST!**: Sprint Straightaways ○ +2, Homestretch Haste +2  
  needs: win Sprinters Stakes; win Takamatsunomiya Kinen; y_dt_gn_race_no_w 3 "dist" 0 "g" 2

## Biwa Hayahide

- **After the Takarazuka Kinen: The Top**: Prepared to Pass +1  
  needs: win Takarazuka Kinen (Senior)
- **Our Method**: Straightaway Acceleration +1  
  needs: triple_crown
- **Unruly Equation**: Hanshin Racecourse ○ +2  
  needs: ev "501023520"; rt_race_w 10009 1

## Copano Rickey

- **After the Kashiwa Kinen: But I Won't Sing**: Sunny Sign +1  
  needs: win Kashiwa Kinen (Senior)
- **Eleven Stars in the Sky**: Top Pick +2, Forward, March! +2  
  needs: win_g1_length 4 11

## Curren Chan

- **Summer Sprint Series Conquered!**: Sprinting Gear +2  
  needs: win Hakodate Sprint Stakes (Senior) [unknown: not a G1 (outside solver data)]; win Keeneland Cup (Senior) [unknown: not a G1 (outside solver data)]; win Centaur Stakes (Senior) [unknown: not a G1 (outside solver data)]

## Daiwa Scarlet

- **After the Tenno Sho (Autumn): They Haven't Changed**: Steadfast +1  
  needs: win Tenno Sho (Autumn) (Senior)
- **Number One and the Coolest**: Unyielding Spirit +2, Head-On +2, Nimble Navigator +1 *  
  needs: win Hanshin Juvenile Fillies; win Oka Sho; win Tokyo Yushun (Japanese Derby) [impossible: clashes with goal Japanese Oaks]; win Takarazuka Kinen (Classic); win Shuka Sho; win Queen Elizabeth II Cup (Classic); win Japan Cup (Classic); win Osaka Hai; win Tenno Sho (Autumn); win Japan Cup (Senior); fires: Senior month 12 late — **impossible in this career**
- **Number One and the Coolest** (pre_2nd_anni): Unyielding Spirit +2, Head-On +2  
  needs: win_all_g1_rival Vodka; win Hanshin Juvenile Fillies; win Oka Sho; win Tokyo Yushun (Japanese Derby) [impossible: clashes with goal Japanese Oaks]; win Takarazuka Kinen (Classic); win Shuka Sho; win Queen Elizabeth II Cup (Classic); win Japan Cup (Classic); win Osaka Hai; win Tenno Sho (Autumn); win Japan Cup (Senior) — **impossible in this career**

## Eishin Flash

- **After the Japanese Derby: Der Sieg**: Fighter +1  
  needs: win Tokyo Yushun (Japanese Derby)
- **Der Rekord**: Tokyo Racecourse ○ +2, Standard Distance ○ +2  
  needs: win_g1_track "10006" "5"; win Arima Kinen (Senior)
- **Erinnerung an Tokyo**: Tokyo Racecourse ○ +2  
  needs: win_g1_track "10006" "2"; win Arima Kinen (Senior)
- **Erinnerung an den Sieg**: Tokyo Racecourse ○ +2  
  needs: win_g1_track "10006" "3"; win Arima Kinen (Senior)

## Fine Motion

- **After the Arima Kinen: Still with You**: Preferred Position +2  
  needs: dont_pick_and_win Arima Kinen (Senior)
- **After the Arima Kinen: Sweet Memories**: Preferred Position +2, Head-On +2  
  needs: pick_and_win Arima Kinen (Senior)
- **After the Mile Ch.: Rival**: Updrafters +2  
  needs: pick_and_win Mile Championship (Senior)
- **After the Queen Elizabeth II Cup: Respect**: Straight Descent +2  
  needs: pick_and_win Queen Elizabeth II Cup (Senior)
- **After the Sapporo Kinen: Home Straight**: Standing By +1  
  needs: win_as_strat Sapporo Kinen (Senior) 4 [unknown: not a G1 (outside solver data)]
- **After the Shuka Sho: Beautiful Sky**: Ramp Up +1 *  
  needs: win Shuka Sho
- **After the Tenno Sho (Autumn): Ever After**: Up-Tempo +2  
  needs: pick_and_win Tenno Sho (Autumn) (Senior)

## Fuji Kiseki

- **Can't Stop the Beat**: Unyielding Spirit +1  
  needs: win_g1_cnt_class_distance 3 3 [0, 1]; win Takamatsunomiya Kinen
- **My Lucky Star**: Preferred Position +2  
  needs: win Tenno Sho (Autumn) (Senior); win Japan Cup (Senior); win Arima Kinen (Senior); win Takarazuka Kinen (Senior)
- **My Lucky Star** (pre_2nd_anni): Preferred Position +1  
  needs: win Tenno Sho (Autumn) (Senior); win Japan Cup (Senior); win Arima Kinen (Senior); win Takarazuka Kinen (Senior)
- **Words Do No Justice**: Prepared to Pass +2  
  needs: 3_crown_route; win Satsuki Sho; win Tokyo Yushun (Japanese Derby); win Kikuka Sho
- **Words Do No Justice** (pre_2nd_anni): Prepared to Pass +1  
  needs: 3_crown_route; win Satsuki Sho; win Tokyo Yushun (Japanese Derby); win Kikuka Sho

## Grass Wonder

- **Beyond the Triple Crown**: Up-Tempo +2  
  needs: win ["100501"]; win ["101001"]; win ["101501"]
- **Beyond the Triple Crown** (pre_2nd_anni [pre_first_anni]): Up-Tempo +1  
  needs: win ["100501"]; win ["101001"]; win ["101501"]

## Haru Urara

- **After the Arima Kinen: Way, Way Better**: Long Shot ○ +5  
  needs: win Arima Kinen (Senior)
- **After the Arima Kinen: Way, Way Better** (pre_2nd_anni): Long Shot ○ +1  
  needs: win Arima Kinen (Senior)

## Hishi Amazon

- **After the Shuka Sho: Triple Tiara**: Steadfast +2  
  needs: win Oka Sho; win Japanese Oaks; win Shuka Sho

## Inari One

- **Behold the Thunder, Wind, and Light**: All I've Got +1, Fighting Spirit +1  
  needs: win_n_of 2 of: Takarazuka Kinen (Classic), Tenno Sho (Autumn) (Classic), Japan Cup (Classic)
- **Come See the Never-Ending Festival!**: Eager +1  
  needs: win_all 4 of: Mainichi Okan (Senior), Tenno Sho (Autumn) (Senior), Japan Cup (Senior), Arima Kinen (Senior)

## Ines Fujin

- **You Are the Wind**: Early Lead +2, Front Runner Straightaways ○ +2, Front Runner Straightaways ○ +1  
  needs: fires: Senior month 12 late

## Kawakami Princess

- **After the Takamatsunomiya Kinen: Buck Up**: Homestretch Haste +1  
  needs: rival_draw Takamatsunomiya Kinen vs King Halo
- **After the Takamatsunomiya Kinen: This Is My Choice**: Homestretch Haste +1  
  needs: win Takamatsunomiya Kinen
- **After the Takamatsunomiya Kinen: Your Image**: Homestretch Haste +1  
  needs: lose_to_rival Takamatsunomiya Kinen vs King Halo
- **Finding the Line between Individuality and Ego**: Homestretch Haste +1  
  needs: do_not_participate Takamatsunomiya Kinen
- **The Umamusume Who Claimed All Tiaras**: Outer Swell +2  
  needs: win Hanshin Juvenile Fillies; win Oka Sho; win Japanese Oaks; win Shuka Sho; win Victoria Mile; win Queen Elizabeth II Cup (Classic); win Queen Elizabeth II Cup (Senior)

## Kitasan Black

- **After the Arima Kinen: Surpassing One's Hero**: Prudent Positioning +2  
  needs: triple_crown; win Arima Kinen (Senior)
- **After the Kikuka Sho: Three Cheers for the Triple Crown Festival!**: Keeping the Lead +2, Firm Resolve +1  
  needs: triple_crown
- **After the Kikuka Sho: Three Cheers for the Triple Crown Festival!** (pre_first_anni): Keeping the Lead +2  
  needs: triple_crown
- **Doing the Step with You!**: Spring Runner ○ +2, Soft Step +2  
  needs: spring_triple_crown

## Manhattan Cafe

- **The Tireless Wonder**: Long Straightaways ○ +1, Long Straightaways ○ +2, Long Straightaways ○ +2, Long Corners ○ +2  
  needs: dist_wins_branch 3 "2+"; fires: Senior month 12 late

## Matikanefukukitaru

- **Imminent Fortune, Quiet Resolve**: Slick Surge +2  
  needs: win Kyoto Shimbun Hai [unknown: not a G1 (outside solver data)]; win Kobe Shimbun Hai [unknown: not a G1 (outside solver data)]; win Kikuka Sho

## Matikanetannhauser

- **After the All Comers: Win and Run Wild**: All I've Got +2  
  needs: beat_rival All Comers (Senior) vs Twin Turbo [unknown: not a G1 (outside solver data)]
- **After the All Comers: Win and Run Wild** (pre_2nd_anni): All I've Got +1  
  needs: beat_rival All Comers (Senior) vs Twin Turbo [unknown: not a G1 (outside solver data)]
- **After the Arima Kinen: Ramen to Wash Down the Win**: Inside Scoop +2  
  needs: beat_rival Arima Kinen (Classic) vs Nice Nature
- **After the Arima Kinen: Ramen to Wash Down the Win** (pre_2nd_anni): Inside Scoop +1  
  needs: beat_rival Arima Kinen (Classic) vs Nice Nature
- **After the Mile Championship: High-Key Happy**: Mile Corners ○ +2  
  needs: beat_rival Mile Championship (Classic) vs Daitaku Helios
- **After the Mile Championship: High-Key Happy** (pre_2nd_anni): Mile Corners ○ +1  
  needs: beat_rival Mile Championship (Classic) vs Daitaku Helios
- **After the Yasuda Kinen: A Triumphant Bath**: Updrafters +2  
  needs: beat_rival Yasuda Kinen (Senior) vs Ikuno Dictus
- **After the Yasuda Kinen: A Triumphant Bath** (pre_2nd_anni): Updrafters +1  
  needs: beat_rival Yasuda Kinen (Senior) vs Ikuno Dictus

## Mayano Top Gun

- **The Fruits of My Labor**: Final Push +1, Head-On +1, Slick Surge +1, Straightaway Spurt +1  
  needs: win_g1_strat 1; win_g1_strat 2; win_g1_strat 3; win_g1_strat 4; fires: Senior month 12 late

## Meisho Doto

- **Triumph! I'm the Overlord?!**: Prepared to Pass +1, Up-Tempo +1  
  needs: win Satsuki Sho; win Tenno Sho (Spring); win Takarazuka Kinen (Senior); win Tenno Sho (Autumn) (Senior); win Japan Cup (Senior); win Arima Kinen (Senior)

## Mejiro Ardan

- **After the Japanese Derby: Shattering Fate**: All I've Got +1  
  needs: win Tokyo Yushun (Japanese Derby)
- **After the Kikuka Sho: Fighting the Current**: Pressure +2  
  needs: win Kikuka Sho
- **After the Tenno Sho (Autumn): Against the Thunder**: Steadfast +2  
  needs: win Tenno Sho (Autumn) (Classic)
- **After the Tenno Sho (Autumn): Now, Lights at Their Zenith**: Corner Recovery ○ +2  
  needs: participate Tenno Sho (Autumn) (Classic); win Tenno Sho (Autumn) (Senior)
- **After the Tenno Sho (Autumn): Now, Vivid Lights Converge**: Ramp Up +2 *  
  needs: participate Kikuka Sho (Classic); win Tenno Sho (Autumn) (Senior)

## Mejiro Bright

- **A Mejiro's Path: Consecutive Queen**: Steadfast +2  
  needs: win Queen Elizabeth II Cup (Classic); win Queen Elizabeth II Cup (Senior)
- **A Mejiro's Path: Dual Party Fever**: Non-Standard Distance ○ +2  
  needs: win Takarazuka Kinen (Senior); win Arima Kinen (Senior)
- **A Mejiro's Path: Two Plaques**: Inside Scoop +2  
  needs: win Tenno Sho (Spring); win Tenno Sho (Autumn) (Senior)
- **Peerless Stayer**: Long Corners ○ +1, Long Corners ○ +2, Long Corners ○ +3, Passing Pro +2, Smoke Screen +1  
  needs: dist_wins_branch 3; fires: Senior month 12 late

## Mejiro Dober

- **From Admiring to Admired**: All I've Got +2  
  needs: win Osaka Hai; win Takarazuka Kinen (Senior); win Queen Elizabeth II Cup (Senior); win_g1_year 3 5
- **Proof of Progress**: Unyielding Spirit +2  
  needs: triple_tiara

## Mejiro Palmer

- **Head Held Hiiiiigh!**: Runaway +1  
  needs: win Arima Kinen (Classic); win Takarazuka Kinen (Senior); win Arima Kinen (Senior)

## Mejiro Ryan

- **A Fresh Wind**: Slick Surge +2  
  needs: triple_crown

## Mihono Bourbon

- **After the Japanese Derby: The Second Crown**: Final Push +1  
  needs: win Tokyo Yushun (Japanese Derby)
- **After the Kikuka Sho: Triple Crown Champion**: Straightaway Acceleration +2  
  needs: triple_crown

## Nakayama Festa

- **After the Takarazuka Kinen: Loud and Proud**: Slick Surge +1  
  needs: win Takarazuka Kinen (Senior)
- **The Gold Beyond the Ship**: Nimble Navigator +2 *, Be Still +2  
  needs: win Kikuka Sho (Classic); win Tenno Sho (Spring) (Senior); win Takarazuka Kinen (Senior); win Tenno Sho (Autumn) (Senior); win Japan Cup (Senior); win Arima Kinen (Senior); fires: Senior month 12 late

## Narita Taishin

- **Forever Fervent**: Early Start +2  
  needs: win Asahi Hai Futurity Stakes; win Satsuki Sho; win Tokyo Yushun (Japanese Derby); win Kikuka Sho; win Japan Cup (Classic); win Arima Kinen (Classic); win Tenno Sho (Spring); win Takarazuka Kinen (Senior); win Tenno Sho (Autumn) (Senior); win Arima Kinen (Senior)

## Nice Nature

- **Choosing to Do My Best**: Homestretch Haste +2  
  needs: win Kyoto Shimbun Hai [unknown: not a G1 (outside solver data)]; win Naruo Kinen (Classic) [unknown: not a G1 (outside solver data)]; win Kokura Kinen (Classic) [unknown: not a G1 (outside solver data)]

## Nishino Flower

- **After the Queen Elizabeth II Cup: Wow!**: Steadfast +1  
  needs: win Queen Elizabeth II Cup
- **After the Takarazuka Kinen: Surrounded by Big Sisters!**: Head-On +1  
  needs: win Takarazuka Kinen (Senior)
- **Mile Go-Getter!**: Updrafters +2, Shifting Gears +2  
  needs: win Hanshin Juvenile Fillies; win Oka Sho; win Milers Cup [unknown: not a G1 (outside solver data)]; win Yasuda Kinen; win Mile Championship (Classic); win Mile Championship (Senior)

## Oguri Cap

- **Together Forever**: Hydrate +1  
  needs: win Kikuka Sho; win Arima Kinen (Classic); win Osaka Hai; win Tenno Sho (Spring); fires: Senior month 12 late

## Rice Shower

- **A Tale That Blooms from the Heart**: Kyoto Racecourse ○ +2, Firm Conditions ○ +2, Kyoto Racecourse ○ +2, Kyoto Racecourse ○ +2  
  needs: racetrack_wins_branch 10008

## Sakura Bakushin O

- **After the Sprinters S.: Successive Wins!**: Moxie +1, Light as a Feather +1  
  needs: win Sprinters Stakes (Classic); win Sprinters Stakes (Senior)
- **After the Sprinters S.: Successive Wins!** (pre_2nd_anni): Moxie +1  
  needs: win Sprinters Stakes (Classic); win Sprinters Stakes (Senior)

## Sakura Chiyono O

- **After the Arima Kinen: Victory**: Feature Act +3  
  needs: win Arima Kinen (Senior)
- **After the Arima Kinen: Victory** (pre_3rd_anni): Feature Act +1  
  needs: win Arima Kinen (Senior)
- **After the Japan Cup: Overcoming Fate**: Head-On +3  
  needs: obj; win Japan Cup (Senior)

## Satono Diamond

- **Join Those You Admire**: Fall Runner ○ +2  
  needs: autumn_triple_crown_senior

## Seeking the Pearl

- **A Flower Crown for the World**: Medium Corners ○ +2, Up-Tempo +2, Eager +2  
  needs: triple_tiara
- **The Mile is All Mine**: Mile Straightaways ○ +2, Mile Corners ○ +2  
  needs: win Hanshin Juvenile Fillies; win Oka Sho; win NHK Mile Cup; win Mile Championship (Classic); win Victoria Mile; win Yasuda Kinen (Senior); win Mile Championship (Senior)

## Seiun Sky

- **Landed That Triple Crown!**: Fast-Paced +2  
  needs: triple_crown

## Silence Suzuka

- **After the Mainichi Okan: Onward, to Light**: Early Lead +1  
  needs: win Mainichi Okan (Senior) [unknown: not a G1 (outside solver data)]
- **An Inspiring Streak**: Runaway +1  
  needs: win_streak_graded 6; win_on_streak Tenno Sho (Autumn) (Senior)

## Smart Falcon

- **After the Kawasaki Kinen: Hearts Aflutter!**: Kawasaki Racecourse ○ +1, Sunny Sign +1  
  needs: win Kawasaki Kinen
- **Center of Everyone's Heart ☆**: Top Pick +3  
  needs: fans_before_finals 120000

## Special Week

- **Rivals in Arms**: Stamina to Spare +2, Slick Surge +2  
  needs: win Satsuki Sho; win Kikuka Sho; win Japan Cup (Classic); win Takarazuka Kinen (Senior); win Arima Kinen (Senior)

## Super Creek

- **Our Dream Lives On**: Prepared to Pass +2  
  needs: win Arima Kinen (Classic); win Tenno Sho (Spring); autumn_triple_crown_senior

## Sweep Tosho

- **After the Takarazuka Kinen: A Strong Umamusume**: Restraint +3  
  needs: win Takarazuka Kinen (Senior)
- **The Secret of the Hat**: End Closer Corners ○ +2  
  needs: triple_tiara; use_strategy 4

## Symboli Rudolf

- **After the Kikuka Sho: Three Crowns Attained**: Be Still +2  
  needs: triple_crown

## Taiki Shuttle

- **Beyond the Mile**: All I've Got +2, No Looking Back +2, Head-On +2  
  needs: win Mile Championship (Classic); win Kinko Sho [unknown: not a G1 (outside solver data)]; win Takarazuka Kinen (Senior); win Mainichi Okan (Senior) [unknown: not a G1 (outside solver data)]; win Tenno Sho (Autumn) (Senior)
- **Beyond the Mile** (pre_2nd_anni [pre_first_anni]): All I've Got +2, Head-On +2  
  needs: win Mile Championship (Classic); win Kinko Sho [unknown: not a G1 (outside solver data)]; win Takarazuka Kinen (Senior); win Mainichi Okan (Senior) [unknown: not a G1 (outside solver data)]; win Tenno Sho (Autumn) (Senior)

## Tamamo Cross

- **A Special Souvenir**: Tail Held High +2 *  
  needs: autumn_triple_crown_senior; win Takarazuka Kinen (Senior)
- **After the Kyoto Kimpai: Luck's on Our Side!**: Kyoto Racecourse ○ +1, Updrafters +1  
  needs: win Kyoto Kimpai [unknown: not a G1 (outside solver data)]
- **After the Nakayama Kimpai: Luck's on Our Side!**: Nakayama Racecourse ○ +1, Up-Tempo +1  
  needs: win Nakayama Kimpai [unknown: not a G1 (outside solver data)]

## Tokai Teio

- **After the Kikuka Sho: Triple Crown Champion!**: Head-On +2  
  needs: triple_crown

## Vodka

- **After the Yasuda Kinen: Consecutive Celebration**: Nimble Navigator +1 *  
  needs: race_w2 Yasuda Kinen
- **After the Yasuda Kinen: Pedal to the Metal!**: Nimble Navigator +1 *  
  needs: win Yasuda Kinen (Senior)
- **The Coolest and Number One**: All I've Got +2, Slick Surge +2, Pedal to the Metal +2  
  needs: win Tulip Sho [unknown: not a G1 (outside solver data)]; win Oka Sho; win Shuka Sho; win Queen Elizabeth II Cup (Classic); win Osaka Hai; win Tenno Sho (Autumn); win Arima Kinen (Classic); win Arima Kinen (Senior)
- **The Coolest and Number One** (pre_2nd_anni): All I've Got +2, Slick Surge +2  
  needs: win Tulip Sho [unknown: not a G1 (outside solver data)]; win Oka Sho; win Shuka Sho; win Queen Elizabeth II Cup (Classic); win Osaka Hai; win Tenno Sho (Autumn); win Arima Kinen (Classic); win Arima Kinen (Senior)

## Winning Ticket

- **Temporary Truce**: Outer Swell +2  
  needs: win_n_of 1 of: Asahi Hai Futurity Stakes (Junior), Satsuki Sho (Classic), Tokyo Yushun (Japanese Derby) (Classic), Kikuka Sho (Classic), Arima Kinen (Classic), Osaka Hai (Senior), Tenno Sho (Spring) (Senior), Takarazuka Kinen (Senior), Arima Kinen (Senior); fires: Senior month 12 late

## Yaeno Muteki

- **Her Proof of Invincibility**: Ramp Up +1 *, Homestretch Haste +1  
  needs: rn_race_w ["Oguri Cap", "Super Creek", "Inari One"] 8

## Yukino Bijin

- **After the Mile Ch. Nambu Hai: Dazzl'n Ties**: Morioka Racecourse ○ +5, Full Tilt +1  
  needs: win M.C. Nambu Hai (Classic)
- **After the Mile Ch. Nambu Hai: Dazzl'n Ties** (pre_2nd_anni): Morioka Racecourse ○ +1, Full Tilt +1  
  needs: win M.C. Nambu Hai (Classic)
- **After the Mile Ch. Nambu Hai: Dazzl'n Ties** (current): Morioka Racecourse ○ +5, Full Tilt +1  
  needs: do_not_win M.C. Nambu Hai (Classic); win M.C. Nambu Hai (Senior)
- **After the Mile Ch. Nambu Hai: Dazzl'n Ties** (pre_2nd_anni): Morioka Racecourse ○ +1, Full Tilt +1  
  needs: do_not_win M.C. Nambu Hai (Classic); win M.C. Nambu Hai (Senior)
- **The Path Beyond the Tiara**: Corner Acceleration ○ +1, Head-On +1  
  needs: triple_tiara; win Hanshin Juvenile Fillies

## 1141 (JP only)

- **(untitled)**: On the Attack +1, 逸る足取り +1  
  needs: spring_triple_crown; autumn_triple_crown_senior; fires: Senior month 12 late
- **(untitled)** (current): 204622 +3  
  needs: fires: Classic month 10 late
- **(untitled)** (current): アグレッシブ +3  
  needs: win Satsuki Sho (Classic) [unknown: uma not in Global data]
- **(untitled)** (current): 204122 +3  
  needs: win Tokyo Yushun (Japanese Derby) (Classic) [unknown: uma not in Global data]
- **(untitled)** (current): 204082 +1  
  needs: win Japan Cup (Senior) [unknown: uma not in Global data]

## Biko Pegasus (JP only)

- **ターフの上のヒーロー**: Fearless +3, Sprint Straightaways ○ +3, Fearless +3, Mile Straightaways ○ +3, Sprint Straightaways ○ +1, Mile Straightaways ○ +1  
  needs: fires: Senior month 12 late

## Cheval Grand (JP only)

- **ある日のお祝い**: Latch On +2  
  needs: triple_crown
- **日本ダービーの後に・世代最強となるも**: 序盤巧者 +3  
  needs: win Tokyo Yushun (Japanese Derby) [unknown: uma not in Global data]
- **皐月賞の後に・迫りくる“巨浪”**: On the Attack +3  
  needs: win Satsuki Sho [unknown: uma not in Global data]
- **菊花賞の後に・ライバルなキミへ向け**: 狙いを定めて +3  
  needs: win Kikuka Sho [unknown: uma not in Global data]

## Daiichi Ruby (JP only)

- **(untitled)**: Sprint Corners ○ +2, Mile Corners ○ +2, Outer Swell +2  
  needs: win Hanshin Juvenile Fillies [unknown: uma not in Global data]; win Oka Sho [unknown: uma not in Global data]; win NHK Mile Cup [unknown: uma not in Global data]; win Yasuda Kinen (Senior) [unknown: uma not in Global data]; win Victoria Mile [unknown: uma not in Global data]; win Sprinters Stakes (Senior) [unknown: uma not in Global data]; win Mile Championship (Senior) [unknown: uma not in Global data]; win Takamatsunomiya Kinen [unknown: uma not in Global data]

## Daitaku Helios (JP only)

- **(untitled)**: Productive Plan +1, Unyielding Spirit +1  
  needs: win Yasuda Kinen (Classic) [unknown: uma not in Global data]; win Yasuda Kinen (Senior) [unknown: uma not in Global data]; win Mile Championship (Classic) [unknown: uma not in Global data]; win Mile Championship (Senior) [unknown: uma not in Global data]; fires: Senior month 12 early
- **(untitled)** (current): Runaway +1  
  needs: win Arima Kinen (Senior) [unknown: uma not in Global data]; fires: Senior month 12 late
- **(untitled)** (current): Runaway +1  
  needs: lose_to_rival Arima Kinen (Senior) vs Mejiro Palmer [unknown: uma not in Global data]; fires: Senior month 12 late
- **(untitled)** (current): Runaway +1  
  needs: rival_draw Arima Kinen (Senior) vs Mejiro Palmer [unknown: uma not in Global data]; fires: Senior month 12 late

## Hishi Miracle (JP only)

- **(untitled)**: Position Pilfer +1, Scramble +1, Start the Charge +1  
  needs: win Tokyo Yushun (Japanese Derby) [unknown: uma not in Global data]; race_w2 Japan Cup [unknown: uma not in Global data]; race_w2 Arima Kinen [unknown: uma not in Global data]; win Takarazuka Kinen (Senior) [unknown: uma not in Global data]; win Tenno Sho (Autumn) (Senior) [unknown: uma not in Global data]; fires: Senior month 12 late

## Hokko Tarumae (JP only)

- **(untitled)**: Sunny Sign +2  
  needs: fires: Senior month 12 late; gn_race_w 100 9; win Tokyo Daishoten [unknown: uma not in Global data]
- **The Days Past and the Days to Come**: Forward, March! +2  
  needs: fires: Senior month 12 late; dt_gn_race_w "trn" 2 100 10

## Katsuragi Ace (JP only)

- **(untitled)**: Mad Dash +1  
  needs: spring_triple_crown; autumn_triple_crown_senior
- **(untitled)** (current): Runaway +1  
  needs: y_gn_race_w 3 "g" 5; win Japan Cup (Senior) [unknown: uma not in Global data]; fires: Senior month 11 late
- **(untitled)** (current): Groundwork +1  
  needs: win Japan Cup (Senior) [unknown: uma not in Global data]

## Mr. C.B. (JP only)

- **(untitled)**: Breakin' Out +2  
  needs: win Japan Cup (Classic) [unknown: uma not in Global data]; win Arima Kinen (Classic) [unknown: uma not in Global data]
- **(untitled)** (current): Ramp Up +2 *  
  needs: ev_cn_on "小さな恋の物語" 2; gn_race_no_w "g" 1

## Neo Universe (JP only)

- **(untitled)**: Slick Surge +2, Pedal to the Metal +2  
  needs: spring_triple_crown

## Sakura Laurel (JP only)

- **(untitled)**: Pedal to the Metal +1, Break the Mold +1  
  needs: triple_crown
- **(untitled)** (current): Be Still +1, Scramble +1  
  needs: spring_triple_crown; autumn_triple_crown_senior
- **(untitled)** (current): Late Surger Corners ○ +3  
  needs: beat_rival Satsuki Sho vs Narita Brian [unknown: uma not in Global data]
- **(untitled)** (current): Slick Surge +3  
  needs: beat_rival Tokyo Yushun (Japanese Derby) vs Narita Brian [unknown: uma not in Global data]
- **(untitled)** (current): Inside Scoop +3  
  needs: beat_rival Kikuka Sho vs Narita Brian [unknown: uma not in Global data]
- **(untitled)** (current): Locked On +3  
  needs: beat_rival Hanshin Daishoten vs Narita Brian [unknown: uma not in Global data]
- **(untitled)** (current): Spring Runner ○ +1  
  needs: win Tenno Sho (Spring) [unknown: uma not in Global data]

## Shinko Windy (JP only)

- **(untitled)**: Top Pick +2  
  needs: win Zen-Nippon Junior Yushun [unknown: uma not in Global data]; win M.C. Nambu Hai [unknown: uma not in Global data]; win February Stakes [unknown: uma not in Global data]; win Kashiwa Kinen [unknown: uma not in Global data]
- **(untitled)** (current): Forward, March! +2  
  needs: ev "501043524"

## Symboli Kris S (JP only)

- **Words Exchanged**: Corner Adept ○ +1, Corner Adept ○ +1, Nimble Navigator +1 *  
  needs: gn_race_w 100 8

## Tanino Gimlet (JP only)

- **(untitled)**: End Closer Straightaways ○ +2, Sharp Steps +2  
  needs: win Tokyo Yushun (Japanese Derby) [unknown: uma not in Global data]; dt_gn_race_w "dist" 1 100 5

## Twin Turbo (JP only)

- **(untitled)**: Up-Tempo +3  
  needs: win Satsuki Sho [unknown: uma not in Global data]
- **(untitled)** (current): Rosy Outlook +3  
  needs: win Tokyo Yushun (Japanese Derby) [unknown: uma not in Global data]
- **(untitled)** (current): Runaway +1  
  needs: win Tanabata Sho [unknown: uma not in Global data]
- **(untitled)** (current): Deep Breaths +2, Nakayama Racecourse ○ +2  
  needs: win Arima Kinen (Senior) [unknown: uma not in Global data]

## アドマイヤグルーヴ (JP only)

- **(untitled)**: むきだしの情熱 +1  
  needs: triple_tiara

## ウインバリアシオン (JP only)

- **あいつがいたから**: むきだしの情熱 +2, むきだしの情熱 +1, むきだしの情熱 +1  
  needs: fires: Senior month 12 late
- **日本ダービーの後に・弾み**: 鬼気迫って +3  
  needs: beat_rival Tokyo Yushun (Japanese Derby) vs オルフェーヴル [unknown: uma not in Global data]
- **日経賞の後に・アンコール**: Breakin' Out +1  
  needs: win Nikkei Sho [unknown: uma not in Global data]
- **皐月賞の後に・天性のスター**: 惜しみなし +3  
  needs: beat_rival Satsuki Sho vs オルフェーヴル [unknown: uma not in Global data]
- **菊花賞の後に・さらに高く**: End Closer Corners ○ +3  
  needs: beat_rival Kikuka Sho vs オルフェーヴル [unknown: uma not in Global data]

## オルフェーヴル (JP only)

- **王の偉業を称える星**: 推力十分 +2, Breakin' Out +2, 推力十分 +2, 推力十分 +2  
  needs: spring_triple_crown; autumn_triple_crown_senior
- **菊花賞の後に・――みっつ**: 不動の心 +1  
  needs: win Kikuka Sho [unknown: uma not in Global data]

## カルストンライトオ (JP only)

- **サマースプリントシリーズと夏の思い出**: Sprinting Gear +1, Summer Runner ○ +1  
  needs: win Ibis Summer Dash (Senior) [unknown: uma not in Global data]; win Centaur Stakes (Senior) [unknown: uma not in Global data]; win_n_of 1 of: Hakodate Sprint Stakes (Senior), Kitakyushu Kinen (Senior), CBC Sho (Senior), Keeneland Cup (Senior)

## シーザリオ (JP only)

- **NHKマイルCの後に・貴方の花道**: 風切り +3  
  needs: win NHK Mile Cup (Classic) [unknown: uma not in Global data]
- **いつまでも心は変わらぬ**: Slick Surge +1  
  needs: win Hanshin Juvenile Fillies [unknown: uma not in Global data]; win Oka Sho [unknown: uma not in Global data]; win Japanese Oaks [unknown: uma not in Global data]; win Shuka Sho [unknown: uma not in Global data]; win Victoria Mile [unknown: uma not in Global data]; race_w2 Queen Elizabeth II Cup [unknown: uma not in Global data]
- **オークスの後に・プロンプター**: Eager +1  
  needs: win Japanese Oaks [unknown: uma not in Global data]

## ジェンティルドンナ (JP only)

- **珠玉の力を献上せよ**: Straight Descent +1, Pace Chaser Corners ○ +2, On the Attack +2, Pace Chaser Corners ○ +2, Straight Descent +2, On the Attack +2  
  needs: fires: Senior month 12 late

## トランセンド (JP only)

- **渾身のディレクターズカット**: Got the Spirit! +1, Full Tilt +1, 砂払い +1  
  needs: win Kawasaki Kinen [unknown: uma not in Global data]; win February Stakes [unknown: uma not in Global data]; win Kashiwa Kinen [unknown: uma not in Global data]; win Teio Sho [unknown: uma not in Global data]; win M.C. Nambu Hai (Senior) [unknown: uma not in Global data]; win JBC Classic (Senior) [unknown: uma not in Global data]; win Champions Cup (Senior) [unknown: uma not in Global data]; win Tokyo Daishoten (Senior) [unknown: uma not in Global data]

## ブエナビスタ (JP only)

- **(untitled)**: Homestretch Haste +2  
  needs: win Takarazuka Kinen (Senior) [unknown: uma not in Global data]; win Tenno Sho (Autumn) (Senior) [unknown: uma not in Global data]; win Japan Cup (Senior) [unknown: uma not in Global data]; win Arima Kinen (Senior) [unknown: uma not in Global data]; win Tenno Sho (Spring) [unknown: uma not in Global data]

## ラインクラフト (JP only)

- **同じ時代で、違うレースを**: Prepared to Pass +2, Graceful Gait +2, 品行方正 +2, Prepared to Pass +2  
  needs: win Oka Sho [unknown: uma not in Global data]; win Japanese Oaks [unknown: uma not in Global data]; win Shuka Sho [unknown: uma not in Global data]; fires: Senior month 12 late

## ラヴズオンリーユー (JP only)

- **桜花賞の後に・開花する愛**: Nimble Navigator +3 *  
  needs: win Oka Sho [unknown: uma not in Global data]
- **秋華賞の後に・同じ時を愛して**: 駆け降り +3  
  needs: win Shuka Sho [unknown: uma not in Global data]

## ヴィブロス (JP only)

- **Let's Gorgeous Party♪**: Late Surger Straightaways ○ +2, Late Surger Corners ○ +2, Slick Surge +2, Slick Surge +1, Slick Surge +1  
  needs: win Hanshin Juvenile Fillies [unknown: uma not in Global data]; win Oka Sho [unknown: uma not in Global data]; win Japanese Oaks [unknown: uma not in Global data]; win Shuka Sho [unknown: uma not in Global data]; win Queen Elizabeth II Cup (Classic) [unknown: uma not in Global data]; win Victoria Mile [unknown: uma not in Global data]; win Queen Elizabeth II Cup (Senior) [unknown: uma not in Global data]; fires: Senior month 12 late

# Index by skill (current versions)

- **204082**: 1141 (JP only): (untitled) +1
- **204122**: 1141 (JP only): (untitled) +3
- **204622**: 1141 (JP only): (untitled) +3
- **All I've Got**: Inari One: Behold the Thunder, Wind, and Light +1; Matikanetannhauser: After the All Comers: Win and Run Wild +2; Mejiro Ardan: After the Japanese Derby: Shattering Fate +1; Mejiro Dober: From Admiring to Admired +2; Taiki Shuttle: Beyond the Mile +2; Vodka: The Coolest and Number One +2
- **Be Still**: Nakayama Festa: The Gold Beyond the Ship +2; Sakura Laurel (JP only): (untitled) +1; Symboli Rudolf: After the Kikuka Sho: Three Crowns Attained +2
- **Break the Mold**: Sakura Laurel (JP only): (untitled) +1
- **Breakin' Out**: Mr. C.B. (JP only): (untitled) +2; オルフェーヴル (JP only): 王の偉業を称える星 +2; ウインバリアシオン (JP only): 日経賞の後に・アンコール +1
- **Corner Acceleration ○**: Yukino Bijin: The Path Beyond the Tiara +1
- **Corner Adept ○**: Air Shakur: After the Kikuka Sho: King of the Game +2; Symboli Kris S (JP only): Words Exchanged +1; Symboli Kris S (JP only): Words Exchanged +1
- **Corner Recovery ○**: Mejiro Ardan: After the Tenno Sho (Autumn): Now, Lights at Their Zenith +2
- **Deep Breaths**: Twin Turbo (JP only): (untitled) +2
- **Eager**: シーザリオ (JP only): オークスの後に・プロンプター +1; Inari One: Come See the Never-Ending Festival! +1; Seeking the Pearl: A Flower Crown for the World +2
- **Early Lead**: Ines Fujin: You Are the Wind +2; Silence Suzuka: After the Mainichi Okan: Onward, to Light +1
- **Early Start**: Narita Taishin: Forever Fervent +2
- **End Closer Corners ○**: Admire Vega: A Synergy of Stars +2; Admire Vega: A Synergy of Stars +2; Admire Vega: A Synergy of Stars +1; Sweep Tosho: The Secret of the Hat +2; ウインバリアシオン (JP only): 菊花賞の後に・さらに高く +3
- **End Closer Straightaways ○**: Admire Vega: A Synergy of Stars +2; Tanino Gimlet (JP only): (untitled) +2
- **Fall Runner ○**: Air Shakur: It's a New Day +2; Satono Diamond: Join Those You Admire +2
- **Fast-Paced**: Seiun Sky: Landed That Triple Crown! +2
- **Fearless**: Biko Pegasus (JP only): ターフの上のヒーロー +3; Biko Pegasus (JP only): ターフの上のヒーロー +3
- **Feature Act**: Sakura Chiyono O: After the Arima Kinen: Victory +3
- **Fighter**: Eishin Flash: After the Japanese Derby: Der Sieg +1
- **Fighting Spirit**: Air Shakur: After the Japan C.: Noise +3; Inari One: Behold the Thunder, Wind, and Light +1
- **Final Push**: Mayano Top Gun: The Fruits of My Labor +1; Mihono Bourbon: After the Japanese Derby: The Second Crown +1
- **Firm Conditions ○**: Rice Shower: A Tale That Blooms from the Heart +2
- **Firm Resolve**: Kitasan Black: After the Kikuka Sho: Three Cheers for the Triple Crown Festival! +1
- **Forward, March!**: Copano Rickey: Eleven Stars in the Sky +2; Hokko Tarumae (JP only): The Days Past and the Days to Come +2; Shinko Windy (JP only): (untitled) +2
- **Front Runner Straightaways ○**: Ines Fujin: You Are the Wind +2; Ines Fujin: You Are the Wind +1
- **Full Tilt**: トランセンド (JP only): 渾身のディレクターズカット +1; Yukino Bijin: After the Mile Ch. Nambu Hai: Dazzl'n Ties +1; Yukino Bijin: After the Mile Ch. Nambu Hai: Dazzl'n Ties +1
- **Got the Spirit!**: トランセンド (JP only): 渾身のディレクターズカット +1
- **Graceful Gait**: ラインクラフト (JP only): 同じ時代で、違うレースを +2
- **Groundwork**: Katsuragi Ace (JP only): (untitled) +1
- **Hanshin Racecourse ○**: Biwa Hayahide: Unruly Equation +2
- **Head-On**: Daiwa Scarlet: Number One and the Coolest +2 — impossible; Fine Motion: After the Arima Kinen: Sweet Memories +2; Mayano Top Gun: The Fruits of My Labor +1; Nishino Flower: After the Takarazuka Kinen: Surrounded by Big Sisters! +1; Sakura Chiyono O: After the Japan Cup: Overcoming Fate +3; Taiki Shuttle: Beyond the Mile +2; Tokai Teio: After the Kikuka Sho: Triple Crown Champion! +2; Yukino Bijin: The Path Beyond the Tiara +1
- **Homestretch Haste**: Bamboo Memory: I'm the BEST OF THE BEST! +2; ブエナビスタ (JP only): (untitled) +2; Kawakami Princess: Finding the Line between Individuality and Ego +1; Kawakami Princess: After the Takamatsunomiya Kinen: This Is My Choice +1; Kawakami Princess: After the Takamatsunomiya Kinen: Your Image +1; Kawakami Princess: After the Takamatsunomiya Kinen: Buck Up +1; Nice Nature: Choosing to Do My Best +2; Yaeno Muteki: Her Proof of Invincibility +1
- **Hydrate**: Oguri Cap: Together Forever +1
- **Inside Scoop**: Matikanetannhauser: After the Arima Kinen: Ramen to Wash Down the Win +2; Mejiro Bright: A Mejiro's Path: Two Plaques +2; Sakura Laurel (JP only): (untitled) +3
- **Kawasaki Racecourse ○**: Smart Falcon: After the Kawasaki Kinen: Hearts Aflutter! +1
- **Keeping the Lead**: Kitasan Black: After the Kikuka Sho: Three Cheers for the Triple Crown Festival! +2
- **Kyoto Racecourse ○**: Rice Shower: A Tale That Blooms from the Heart +2; Rice Shower: A Tale That Blooms from the Heart +2; Rice Shower: A Tale That Blooms from the Heart +2; Tamamo Cross: After the Kyoto Kimpai: Luck's on Our Side! +1
- **Latch On**: Cheval Grand (JP only): ある日のお祝い +2
- **Late Surger Corners ○**: Sakura Laurel (JP only): (untitled) +3; ヴィブロス (JP only): Let's Gorgeous Party♪ +2
- **Late Surger Straightaways ○**: ヴィブロス (JP only): Let's Gorgeous Party♪ +2
- **Light as a Feather**: Sakura Bakushin O: After the Sprinters S.: Successive Wins! +1
- **Locked On**: Sakura Laurel (JP only): (untitled) +3
- **Long Corners ○**: Manhattan Cafe: The Tireless Wonder +2; Mejiro Bright: Peerless Stayer +1; Mejiro Bright: Peerless Stayer +2; Mejiro Bright: Peerless Stayer +3
- **Long Shot ○**: Haru Urara: After the Arima Kinen: Way, Way Better +5
- **Long Straightaways ○**: Manhattan Cafe: The Tireless Wonder +1; Manhattan Cafe: The Tireless Wonder +2; Manhattan Cafe: The Tireless Wonder +2
- **Mad Dash**: Katsuragi Ace (JP only): (untitled) +1
- **Medium Corners ○**: Seeking the Pearl: A Flower Crown for the World +2
- **Mile Corners ○**: Daiichi Ruby (JP only): (untitled) +2; Matikanetannhauser: After the Mile Championship: High-Key Happy +2; Seeking the Pearl: The Mile is All Mine +2
- **Mile Straightaways ○**: Biko Pegasus (JP only): ターフの上のヒーロー +3; Biko Pegasus (JP only): ターフの上のヒーロー +1; Seeking the Pearl: The Mile is All Mine +2
- **Morioka Racecourse ○**: Agnes Digital: After the Mile Ch. Nambu Hai: This Is Heaven +1; Yukino Bijin: After the Mile Ch. Nambu Hai: Dazzl'n Ties +5; Yukino Bijin: After the Mile Ch. Nambu Hai: Dazzl'n Ties +5
- **Moxie**: Sakura Bakushin O: After the Sprinters S.: Successive Wins! +1
- **Nakayama Racecourse ○**: Tamamo Cross: After the Nakayama Kimpai: Luck's on Our Side! +1; Twin Turbo (JP only): (untitled) +2
- **Nimble Navigator**: Daiwa Scarlet: Number One and the Coolest +1 — impossible; ラヴズオンリーユー (JP only): 桜花賞の後に・開花する愛 +3; Nakayama Festa: The Gold Beyond the Ship +2; Symboli Kris S (JP only): Words Exchanged +1; Vodka: After the Yasuda Kinen: Pedal to the Metal! +1; Vodka: After the Yasuda Kinen: Consecutive Celebration +1
- **No Looking Back**: Taiki Shuttle: Beyond the Mile +2
- **Non-Standard Distance ○**: Mejiro Bright: A Mejiro's Path: Dual Party Fever +2
- **On the Attack**: Cheval Grand (JP only): 皐月賞の後に・迫りくる“巨浪” +3; 1141 (JP only): (untitled) +1; ジェンティルドンナ (JP only): 珠玉の力を献上せよ +2; ジェンティルドンナ (JP only): 珠玉の力を献上せよ +2
- **Outer Swell**: Daiichi Ruby (JP only): (untitled) +2; Kawakami Princess: The Umamusume Who Claimed All Tiaras +2; Winning Ticket: Temporary Truce +2
- **Pace Chaser Corners ○**: ジェンティルドンナ (JP only): 珠玉の力を献上せよ +2; ジェンティルドンナ (JP only): 珠玉の力を献上せよ +2
- **Passing Pro**: Mejiro Bright: Peerless Stayer +2
- **Pedal to the Metal**: Neo Universe (JP only): (untitled) +2; Sakura Laurel (JP only): (untitled) +1; Vodka: The Coolest and Number One +2
- **Position Pilfer**: Hishi Miracle (JP only): (untitled) +1
- **Preferred Position**: Fine Motion: After the Arima Kinen: Sweet Memories +2; Fine Motion: After the Arima Kinen: Still with You +2; Fuji Kiseki: My Lucky Star +2
- **Prepared to Pass**: Biwa Hayahide: After the Takarazuka Kinen: The Top +1; Fuji Kiseki: Words Do No Justice +2; Meisho Doto: Triumph! I'm the Overlord?! +1; ラインクラフト (JP only): 同じ時代で、違うレースを +2; ラインクラフト (JP only): 同じ時代で、違うレースを +2; Super Creek: Our Dream Lives On +2
- **Pressure**: Mejiro Ardan: After the Kikuka Sho: Fighting the Current +2
- **Productive Plan**: Aston Machan: Say It to Machan, Too +2; Daitaku Helios (JP only): (untitled) +1
- **Prudent Positioning**: Kitasan Black: After the Arima Kinen: Surpassing One's Hero +2
- **Ramp Up**: Fine Motion: After the Shuka Sho: Beautiful Sky +1; Mejiro Ardan: After the Tenno Sho (Autumn): Now, Vivid Lights Converge +2; Mr. C.B. (JP only): (untitled) +2; Yaeno Muteki: Her Proof of Invincibility +1
- **Rapid Gain**: Agnes Digital: After the Mile Ch. Nambu Hai: This Is Heaven +1
- **Restraint**: Sweep Tosho: After the Takarazuka Kinen: A Strong Umamusume +3
- **Rosy Outlook**: Aston Machan: After the Shuka Sho: Machan's Here Too +2; Twin Turbo (JP only): (untitled) +3
- **Runaway**: Daitaku Helios (JP only): (untitled) +1; Daitaku Helios (JP only): (untitled) +1; Daitaku Helios (JP only): (untitled) +1; Katsuragi Ace (JP only): (untitled) +1; Mejiro Palmer: Head Held Hiiiiigh! +1; Silence Suzuka: An Inspiring Streak +1; Twin Turbo (JP only): (untitled) +1
- **Scramble**: Hishi Miracle (JP only): (untitled) +1; Sakura Laurel (JP only): (untitled) +1
- **Sharp Steps**: Tanino Gimlet (JP only): (untitled) +2
- **Shifting Gears**: Aston Machan: Tell Machan +2; Nishino Flower: Mile Go-Getter! +2
- **Slick Surge**: シーザリオ (JP only): いつまでも心は変わらぬ +1; Matikanefukukitaru: Imminent Fortune, Quiet Resolve +2; Mayano Top Gun: The Fruits of My Labor +1; Mejiro Ryan: A Fresh Wind +2; Nakayama Festa: After the Takarazuka Kinen: Loud and Proud +1; Neo Universe (JP only): (untitled) +2; Sakura Laurel (JP only): (untitled) +3; Special Week: Rivals in Arms +2; ヴィブロス (JP only): Let's Gorgeous Party♪ +2; ヴィブロス (JP only): Let's Gorgeous Party♪ +1; ヴィブロス (JP only): Let's Gorgeous Party♪ +1; Vodka: The Coolest and Number One +2
- **Smoke Screen**: Mejiro Bright: Peerless Stayer +1
- **Soft Step**: Kitasan Black: Doing the Step with You! +2
- **Spring Runner ○**: Air Shakur: Just as Planned! +2; Kitasan Black: Doing the Step with You! +2; Sakura Laurel (JP only): (untitled) +1
- **Sprint Corners ○**: Daiichi Ruby (JP only): (untitled) +2
- **Sprint Straightaways ○**: Bamboo Memory: I'm the BEST OF THE BEST! +2; Biko Pegasus (JP only): ターフの上のヒーロー +3; Biko Pegasus (JP only): ターフの上のヒーロー +1
- **Sprinting Gear**: カルストンライトオ (JP only): サマースプリントシリーズと夏の思い出 +1; Curren Chan: Summer Sprint Series Conquered! +2
- **Stamina to Spare**: Special Week: Rivals in Arms +2
- **Standard Distance ○**: Eishin Flash: Der Rekord +2
- **Standing By**: Fine Motion: After the Sapporo Kinen: Home Straight +1
- **Start the Charge**: Hishi Miracle (JP only): (untitled) +1
- **Steadfast**: Daiwa Scarlet: After the Tenno Sho (Autumn): They Haven't Changed +1; Hishi Amazon: After the Shuka Sho: Triple Tiara +2; Mejiro Ardan: After the Tenno Sho (Autumn): Against the Thunder +2; Mejiro Bright: A Mejiro's Path: Consecutive Queen +2; Nishino Flower: After the Queen Elizabeth II Cup: Wow! +1
- **Straight Descent**: Fine Motion: After the Queen Elizabeth II Cup: Respect +2; ジェンティルドンナ (JP only): 珠玉の力を献上せよ +1; ジェンティルドンナ (JP only): 珠玉の力を献上せよ +2
- **Straightaway Acceleration**: Biwa Hayahide: Our Method +1; Mihono Bourbon: After the Kikuka Sho: Triple Crown Champion +2
- **Straightaway Spurt**: Mayano Top Gun: The Fruits of My Labor +1
- **Summer Runner ○**: カルストンライトオ (JP only): サマースプリントシリーズと夏の思い出 +1
- **Sunny Sign**: Copano Rickey: After the Kashiwa Kinen: But I Won't Sing +1; Hokko Tarumae (JP only): (untitled) +2; Smart Falcon: After the Kawasaki Kinen: Hearts Aflutter! +1
- **Tactical Tweak**: Agnes Tachyon: The Triple Crown Champion That Was +2
- **Tail Held High**: Tamamo Cross: A Special Souvenir +2
- **Tokyo Racecourse ○**: Eishin Flash: Erinnerung an Tokyo +2; Eishin Flash: Erinnerung an den Sieg +2; Eishin Flash: Der Rekord +2
- **Top Pick**: Copano Rickey: Eleven Stars in the Sky +2; Shinko Windy (JP only): (untitled) +2; Smart Falcon: Center of Everyone's Heart ☆ +3
- **Unyielding Spirit**: Agnes Digital: All-Round Ruler +1; Daitaku Helios (JP only): (untitled) +1; Daiwa Scarlet: Number One and the Coolest +2 — impossible; Fuji Kiseki: Can't Stop the Beat +1; Mejiro Dober: Proof of Progress +2
- **Up-Tempo**: Aston Machan: After the Tenno Sho (Autumn): A Legendary Clash +2; Fine Motion: After the Tenno Sho (Autumn): Ever After +2; Grass Wonder: Beyond the Triple Crown +2; Meisho Doto: Triumph! I'm the Overlord?! +1; Seeking the Pearl: A Flower Crown for the World +2; Tamamo Cross: After the Nakayama Kimpai: Luck's on Our Side! +1; Twin Turbo (JP only): (untitled) +3
- **Updrafters**: Agnes Digital: All-Round Ruler +1; Fine Motion: After the Mile Ch.: Rival +2; Matikanetannhauser: After the Yasuda Kinen: A Triumphant Bath +2; Nishino Flower: Mile Go-Getter! +2; Tamamo Cross: After the Kyoto Kimpai: Luck's on Our Side! +1
- **むきだしの情熱**: アドマイヤグルーヴ (JP only): (untitled) +1; ウインバリアシオン (JP only): あいつがいたから +2; ウインバリアシオン (JP only): あいつがいたから +1; ウインバリアシオン (JP only): あいつがいたから +1
- **アグレッシブ**: 1141 (JP only): (untitled) +3
- **不動の心**: オルフェーヴル (JP only): 菊花賞の後に・――みっつ +1
- **品行方正**: ラインクラフト (JP only): 同じ時代で、違うレースを +2
- **序盤巧者**: Cheval Grand (JP only): 日本ダービーの後に・世代最強となるも +3
- **惜しみなし**: ウインバリアシオン (JP only): 皐月賞の後に・天性のスター +3
- **推力十分**: オルフェーヴル (JP only): 王の偉業を称える星 +2; オルフェーヴル (JP only): 王の偉業を称える星 +2; オルフェーヴル (JP only): 王の偉業を称える星 +2
- **狙いを定めて**: Cheval Grand (JP only): 菊花賞の後に・ライバルなキミへ向け +3
- **砂払い**: トランセンド (JP only): 渾身のディレクターズカット +1
- **逸る足取り**: 1141 (JP only): (untitled) +1
- **風切り**: シーザリオ (JP only): NHKマイルCの後に・貴方の花道 +3
- **駆け降り**: ラヴズオンリーユー (JP only): 秋華賞の後に・同じ時を愛して +3
- **鬼気迫って**: ウインバリアシオン (JP only): 日本ダービーの後に・弾み +3
