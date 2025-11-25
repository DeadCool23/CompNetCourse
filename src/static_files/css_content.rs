pub fn get_css() -> String {
    r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    line-height: 1.6;
    color: #333;
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
}

header {
    text-align: center;
    padding: 40px 0;
    background: linear-gradient(135deg, #2c3e50 0%, #3498db 100%);
    color: white;
    border-radius: 10px;
    margin-bottom: 30px;
    box-shadow: 0 4px 6px rgba(0,0,0,0.1);
}

header h1 {
    font-size: 3em;
    margin-bottom: 10px;
    text-shadow: 2px 2px 4px rgba(0,0,0,0.3);
}

header p {
    font-size: 1.2em;
    opacity: 0.9;
}

.main-nav {
    display: flex;
    justify-content: center;
    gap: 20px;
    margin-bottom: 40px;
    flex-wrap: wrap;
}

.main-nav a {
    text-decoration: none;
    color: #2c3e50;
    padding: 10px 20px;
    border: 2px solid #3498db;
    border-radius: 25px;
    transition: all 0.3s ease;
    font-weight: bold;
}

.main-nav a:hover {
    background: #3498db;
    color: white;
    transform: translateY(-2px);
}

.section {
    background: white;
    margin-bottom: 30px;
    padding: 30px;
    border-radius: 10px;
    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
}

.section h2 {
    color: #2c3e50;
    margin-bottom: 20px;
    font-size: 2em;
    border-bottom: 3px solid #3498db;
    padding-bottom: 10px;
}

.content p {
    margin-bottom: 15px;
    font-size: 1.1em;
    line-height: 1.8;
}

.chess-rules {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 30px;
}

.rule {
    background: #f8f9fa;
    padding: 20px;
    border-radius: 8px;
    border-left: 4px solid #3498db;
}

.rule h3 {
    color: #2c3e50;
    margin-bottom: 15px;
}

.rule ul {
    list-style-position: inside;
}

.rule li {
    margin-bottom: 8px;
    padding-left: 10px;
}

.chess-pieces {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 20px;
    text-align: center;
}

.piece {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    padding: 20px;
    border-radius: 10px;
    transition: transform 0.3s ease;
}

.piece:hover {
    transform: scale(1.05);
}

.piece-icon {
    font-size: 3em;
    margin-bottom: 10px;
}

footer {
    text-align: center;
    padding: 20px;
    background: #2c3e50;
    color: white;
    border-radius: 10px;
    margin-top: 40px;
}

@media (max-width: 768px) {
    .container {
        padding: 10px;
    }
    
    header h1 {
        font-size: 2em;
    }
    
    .main-nav {
        flex-direction: column;
        align-items: center;
    }
    
    .chess-rules {
        grid-template-columns: 1fr;
    }
}"#
    .to_string()
}
