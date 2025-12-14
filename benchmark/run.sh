#!/bin/bash

URL="http://127.0.0.1:9898/test.txt"
TEST_FILE="test.txt"
RESULTS_FILE="./results/benchmark_results.csv"
CONCURRENCY_LEVELS="1 10 50 100 500"
FILE_SIZES="1KB 512KB 1MB 64MB 128MB"
REQUEST_TIME=10

if [ -f "$RESULTS_FILE" ]; then
    echo "Файл $RESULTS_FILE уже существует."
    echo "Хотите перегенерировать данные? (y/n)"
    read -r response
    
    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo "Перегенерируем данные..."
        rm -f "$RESULTS_FILE"
        GENERATE_NEW_DATA=true
    else
        echo "Используем существующие данные."
        GENERATE_NEW_DATA=false
    fi
else
    GENERATE_NEW_DATA=true
fi

generate_benchmark_data() {
    mkdir -p results
    echo "file_size,concurrency,requests_per_second" > "$RESULTS_FILE"
    
    size_to_bytes() {
        local size=$1
        local num=${size//[^0-9]/}
        local unit=${size//[0-9]/}
        
        case $unit in
            KB) echo $((num * 1024)) ;;
            MB) echo $((num * 1024 * 1024)) ;;
            *) echo $num ;;
        esac
    }
    
    check_dependencies() {
        local missing=()
        
        if ! command -v ab &> /dev/null; then
            missing+=("apache2-utils (ab)")
        fi
        
        if ! command -v truncate &> /dev/null; then
            missing+=("truncate")
        fi
        
        if [ ${#missing[@]} -gt 0 ]; then
            echo "Ошибка: отсутствуют необходимые утилиты:"
            for dep in "${missing[@]}"; do
                echo "  - $dep"
            done
            echo ""
            return 1
        fi
        return 0
    }
    
    if ! check_dependencies; then
        return 1
    fi
    
    echo "Запуск тестов производительности..."
    echo "====================================="
    
    for size in $FILE_SIZES; do
        echo ""
        echo "Тестирование с размером файла: $size"
        echo "-------------------------------------"
        
        bytes=$(size_to_bytes "$size")
        
        echo "Создание файла размером $size ($bytes байт)..."
        truncate -s "$size" "$TEST_FILE"
        
        echo "Заполнение файла данными..."
        head -c "$bytes" /dev/urandom > "$TEST_FILE" 2>/dev/null
        
        sleep 2
        
        for concurrency in $CONCURRENCY_LEVELS; do
            echo "  Тестирование с concurrency: $concurrency"
            
            output=$(ab -t "$REQUEST_TIME" -c "$concurrency" "$URL" 2>/dev/null)
            
            rps=$(echo "$output" | grep "Requests per second:" | awk '{print $4}')
            
            if [ -n "$rps" ]; then
                echo "$size,$concurrency,$rps" >> "$RESULTS_FILE"
                echo "    RPS: $rps"
            else
                echo "$size,$concurrency,ERROR" >> "$RESULTS_FILE"
                echo "    Ошибка при выполнении теста"
            fi
            
            sleep 1
        done
    done
    
    echo ""
    echo "====================================="
    echo "Тестирование завершено!"
    echo "Результаты сохранены в: $RESULTS_FILE"
}

# Основной скрипт
if [ "$GENERATE_NEW_DATA" = true ]; then
    generate_benchmark_data
fi

if [ -f "$RESULTS_FILE" ]; then
    echo ""
    echo "Визуализация результатов..."
    echo "==========================="
    
    python3 ./benchmark/plot.py --csv "$RESULTS_FILE"
    mv benchmark_*_plot.png ./results/
else
    echo "Файл $RESULTS_FILE не найден. Нечего визуализировать."
fi
