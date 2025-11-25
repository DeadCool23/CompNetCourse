pub fn get_html() -> String {
    String::from("<!DOCTYPE html>
<html lang=\"ru\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>Шахматный клуб</title>
    <link rel=\"stylesheet\" href=\"style.css\">
</head>
<body>
    <div class=\"container\">
        <header>
            <h1>♔ Шахматный клуб ♚</h1>
            <p>Искусство стратегии и тактики</p>
        </header>

        <nav class=\"main-nav\">
            <a href=\"#about\">О шахматах</a>
            <a href=\"#history\">История</a>
            <a href=\"#rules\">Правила</a>
            <a href=\"#gallery\">Галерея</a>
        </nav>

        <section id=\"about\" class=\"section\">
            <h2>О шахматах</h2>
            <div class=\"content\">
                <p>Шахматы — это настольная логическая игра для двух игроков, сочетающая в себе элементы искусства, науки и спорта.</p>
                <p>Игра ведется на квадратной доске, разделенной на 64 клетки, с использованием 32 фигур: 16 белых и 16 черных.</p>
            </div>
        </section>

        <section id=\"history\" class=\"section\">
            <h2>История шахмат</h2>
            <div class=\"content\">
                <p>Шахматы появились в Индии в V-VI веках нашей эры и первоначально назывались \"чатуранга\".</p>
                <p>Через Персию игра попала в арабский мир, а затем в Европу, где приобрела современный вид.</p>
            </div>
        </section>

        <section id=\"rules\" class=\"section\">
            <h2>Основные правила</h2>
            <div class=\"chess-rules\">
                <div class=\"rule\">
                    <h3>Цель игры</h3>
                    <p>Поставить мат королю противника</p>
                </div>
                <div class=\"rule\">
                    <h3>Ходы фигур</h3>
                    <ul>
                        <li>Пешка: на 1 клетку вперед, бьет по диагонали</li>
                        <li>Ладья: по горизонтали и вертикали</li>
                        <li>Конь: буквой \"Г\"</li>
                        <li>Слон: по диагонали</li>
                        <li>Ферзь: в любом направлении</li>
                        <li>Король: на 1 клетку в любом направлении</li>
                    </ul>
                </div>
            </div>
        </section>

        <section id=\"gallery\" class=\"section\">
            <h2>Шахматные фигуры</h2>
            <div class=\"chess-pieces\">
                <div class=\"piece\">
                    <div class=\"piece-icon\">♙</div>
                    <span>Пешка</span>
                </div>
                <div class=\"piece\">
                    <div class=\"piece-icon\">♖</div>
                    <span>Ладья</span>
                </div>
                <div class=\"piece\">
                    <div class=\"piece-icon\">♘</div>
                    <span>Конь</span>
                </div>
                <div class=\"piece\">
                    <div class=\"piece-icon\">♗</div>
                    <span>Слон</span>
                </div>
                <div class=\"piece\">
                    <div class=\"piece-icon\">♕</div>
                    <span>Ферзь</span>
                </div>
                <div class=\"piece\">
                    <div class=\"piece-icon\">♔</div>
                    <span>Король</span>
                </div>
            </div>
        </section>

        <footer>
            <p>&copy; 2025 Шахматный клуб</p>
        </footer>
    </div>
</body>
</html>")
}