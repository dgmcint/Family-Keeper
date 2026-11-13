# Translation glossary

The key words in PAF Classic and Family Keeper, so every screen and both programs use the same
term. English wording follows the PAF 5 User's Guide. Ordinance names follow the terms The Church
of Jesus Christ of Latter-day Saints uses in each language.

Each language file is a standard gettext `.po` file and can be edited with a tool such as
[Poedit](https://poedit.net). `template.pot` lists every phrase in English.

Regional files (`es-419.po`, `pt-PT.po`) list only the phrases that differ from the main
language (`es.po`, `pt-BR.po`).

| English | Español | Français | Norsk | Português (BR) | Português (PT) | Русский |
|---|---|---|---|---|---|---|
| Family | Familia | Famille | Familie | Família | Família | Семья |
| Individual | Persona | Personne | Person | Pessoa | Pessoa | Человек |
| Marriage | Matrimonio | Mariage | Ekteskap | Casamento | Casamento | Брак |
| Spouse | Cónyuge | Conjoint | Ektefelle | Cônjuge | Cônjuge | Супруг(а) |
| Children | Hijos | Enfants | Barn | Filhos | Filhos | Дети |
| Parents | Padres | Parents | Foreldre | Pais | Pais | Родители |
| Pedigree (screen, chart) | Linaje | Arbre (tableau d'ascendance) | Anetavle | Gráfico de linhagem | Gráfico de linhagem | Древо |
| Family Group (record) | Registro de grupo familiar | Fiche familiale | Familiegruppeskjema | Registro de grupo familiar | Registo de grupo familiar | Семейная группа |
| Ancestry / ancestors | Ascendencia / antepasados | Ascendance / ancêtres | Aner | Ascendência / antepassados | Ascendência / antepassados | Предки |
| Descendants | Descendientes | Descendants | Etterkommere | Descendentes | Descendentes | Потомки |
| Birth | Nacimiento | Naissance | Fødsel | Nascimento | Nascimento | Рождение |
| Christening | Bautizo | Baptême (chrétien) | Dåp (kristning) | Batizado | Batizado | Крещение (церковное) |
| Death | Defunción | Décès | Død | Óbito | Óbito | Смерть |
| Burial | Sepultura | Inhumation | Begravelse | Sepultamento | Sepultura | Погребение |
| Source | Fuente | Source | Kilde | Fonte | Fonte | Источник |
| Repository | Depósito | Dépôt | Oppbevaringssted | Repositório | Repositório | Хранилище |
| Notes | Notas | Notes | Notater | Notas | Notas | Заметки |
| Multimedia | Multimedia | Multimédia | Multimedia | Multimídia | Multimédia | Мультимедиа |
| RIN / MRIN | RIN / MRIN | RIN / MRIN | RIN / MRIN | RIN / MRIN | RIN / MRIN | RIN / MRIN |
| Match/Merge | Comparar/Combinar | Comparer/Fusionner | Sammenlign/slå sammen | Comparar/Combinar | Comparar/Combinar | Сравнить/объединить |
| Check/Repair | Verificar/Reparar | Vérifier/Réparer | Kontroller/reparer | Verificar/Reparar | Verificar/Reparar | Проверка/исправление |
| Focus/Filter | Enfoque/Filtro | Focus/Filtre | Fokus/filter | Foco/Filtro | Foco/Filtro | Фокус/фильтр |
| Scrapbook | Álbum de recortes | Album | Utklippsbok | Álbum de recortes | Álbum de recortes | Альбом |
| LDS (Latter-day Saint) | SUD | SDJ | SDH | SUD | SUD | СПД |
| Baptism (LDS) | Bautismo | Baptême | Dåp | Batismo | Batismo | Крещение |
| Confirmation | Confirmación | Confirmation | Konfirmasjon | Confirmação | Confirmação | Конфирмация |
| Endowment | Investidura | Dotation | Begavelse | Investidura | Investidura | Облечение |
| Sealing | Sellamiento | Scellement | Besegling | Selamento | Selamento | Запечатывание |
| Sealed to parents | Sellado a los padres | Scellé aux parents | Beseglet til foreldre | Selado aos pais | Selado aos pais | Запечатан(а) с родителями |
| Sealed to spouse | Sellado al cónyuge | Scellé au conjoint | Beseglet til ektefelle | Selado ao cônjuge | Selado ao cônjuge | Запечатан(а) с супругом |
| Temple | Templo | Temple | Tempel | Templo | Templo | Храм |
| Sex letters M / F / unknown | M / F / D | M / F / I | M / K / U | M / F / D | M / F / D | М / Ж / Н |

German, Swedish, Japanese, Korean and Traditional Chinese came from the original program's
language files; their terms are kept as they were.

## Things that are not translated

- Family data: names, dates and places as typed.
- GEDCOM date words such as ABT, BEF and the month abbreviations, which are part of the date.
- RIN, MRIN and AFN.
- Printed reports (planned for later).

## Placeholders

Words in braces, such as `{name}` or `{n}`, are filled in by the program. Keep them exactly as
they are; they can move anywhere in the sentence. Phrases with a count have one form per plural
form of the language (one for Japanese, Korean and Chinese; two for most languages; three for
Russian).
