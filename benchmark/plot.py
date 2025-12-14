#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import sys
from pathlib import Path
import argparse

def read_benchmark_data(csv_file):
    try:
        df = pd.read_csv(csv_file)
        return df
    except Exception as e:
        print(f"Ошибка при чтении файла {csv_file}: {e}")
        sys.exit(1)

def create_benchmark_plot(df, output_file="benchmark_plot.png", show_plot=False):
    if df.empty:
        print("CSV файл пуст или не содержит данных")
        return
    
    required_columns = ['file_size', 'concurrency', 'requests_per_second']
    for col in required_columns:
        if col not in df.columns:
            print(f"Ошибка: в CSV файле отсутствует колонка '{col}'")
            print(f"Доступные колонки: {list(df.columns)}")
            return
    
    if df['requests_per_second'].dtype == 'object':
        df['requests_per_second'] = pd.to_numeric(df['requests_per_second'], errors='coerce')
    
    file_sizes = df['file_size'].unique()
    
    colors = plt.cm.Set2(np.linspace(0, 1, len(file_sizes)))
    
    plt.figure(figsize=(12, 8))
    
    for i, file_size in enumerate(file_sizes):
        size_data = df[df['file_size'] == file_size].copy()
        
        size_data = size_data.sort_values('concurrency')
        
        if size_data['requests_per_second'].isna().any():
            size_data['requests_per_second'] = size_data['requests_per_second'].interpolate()
        
        plt.plot(size_data['concurrency'], 
                size_data['requests_per_second'], 
                marker='o', 
                linewidth=2, 
                markersize=8,
                label=f'{file_size}',
                color=colors[i])
        
        for _, row in size_data.iterrows():
            if not pd.isna(row['requests_per_second']):
                plt.annotate(f"{row['requests_per_second']:.1f}", 
                           (row['concurrency'], row['requests_per_second']),
                           textcoords="offset points",
                           xytext=(0,10),
                           ha='center',
                           fontsize=8)
    
    plt.xlabel('Количество пользователей (concurrency)', fontsize=12, fontweight='bold')
    plt.ylabel('Запросов в секунду (RPS)', fontsize=12, fontweight='bold')
    plt.title('Производительность веб-сервера\nЗависимость RPS от количества одновременных пользователей', 
              fontsize=14, fontweight='bold', pad=20)
    
    plt.xscale('log')
    plt.xticks([1, 10, 50, 100, 500], ['1', '10', '50', '100', '500'])
    
    plt.grid(True, alpha=0.3, linestyle='--')
    plt.grid(True, which='minor', alpha=0.1, linestyle=':')
    
    plt.legend(title='Размер файла', title_fontsize=12, fontsize=10, 
               loc='best', framealpha=0.95)
    
    max_rps = df['requests_per_second'].max()
    min_rps = df['requests_per_second'].min()
    plt.ylim(max(0, min_rps * 0.9), max_rps * 1.1)
    
    total_tests = len(df)
    successful_tests = df['requests_per_second'].notna().sum()
    
    info_text = f"Всего тестов: {total_tests}\nУспешных: {successful_tests}"
    if total_tests > successful_tests:
        info_text += f"\nОшибок: {total_tests - successful_tests}"
    
    plt.figtext(0.02, 0.02, info_text, fontsize=9, 
                bbox=dict(boxstyle="round,pad=0.5", facecolor="lightgray", alpha=0.8))
    
    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    print(f"График сохранен как: {output_file}")
    
    if show_plot:
        plt.show()
    
    return output_file

def create_grouped_bar_chart(df, output_file="benchmark_bar_chart.png", show_plot=False):
    pivot_df = df.pivot(index='concurrency', columns='file_size', values='requests_per_second')
    
    plt.figure(figsize=(14, 8))
    
    x = np.arange(len(pivot_df.index))
    width = 0.15
    colors = plt.cm.tab10(np.linspace(0, 1, len(pivot_df.columns)))
    
    for i, (file_size, color) in enumerate(zip(pivot_df.columns, colors)):
        values = pivot_df[file_size].values
        offset = width * (i - (len(pivot_df.columns) - 1) / 2)
        bars = plt.bar(x + offset, values, width, label=file_size, color=color, alpha=0.8)
        
        for bar in bars:
            height = bar.get_height()
            if not np.isnan(height):
                plt.text(bar.get_x() + bar.get_width()/2., height + 0.02 * max(values),
                        f'{height:.1f}', ha='center', va='bottom', fontsize=8, rotation=90)
    
    plt.xlabel('Количество пользователей (concurrency)', fontsize=12, fontweight='bold')
    plt.ylabel('Запросов в секунду (RPS)', fontsize=12, fontweight='bold')
    plt.title('Сравнение производительности для разных размеров файлов', 
              fontsize=14, fontweight='bold', pad=20)
    
    plt.xticks(x, pivot_df.index)
    plt.legend(title='Размер файла', title_fontsize=12, fontsize=10, 
               loc='upper right', framealpha=0.95)
    
    plt.grid(True, alpha=0.3, axis='y', linestyle='--')
    
    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    print(f"Столбчатая диаграмма сохранена как: {output_file}")
    
    if show_plot:
        plt.show()

def main():
    parser = argparse.ArgumentParser(description='Визуализация результатов Apache Benchmark')
    parser.add_argument('--csv', default='benchmark_results.csv',
                       help='Путь к CSV файлу с результатами (по умолчанию: benchmark_results.csv)')
    parser.add_argument('--output', default='benchmark_plot.png',
                       help='Имя файла для сохранения графика (по умолчанию: benchmark_plot.png)')
    parser.add_argument('--show', action='store_true',
                       help='Показать график после создания')
    parser.add_argument('--type', choices=['line', 'bar', 'both'], default='both',
                       help='Тип графика: line (линейный), bar (столбчатый), both (оба)')
    
    args = parser.parse_args()
    
    csv_path = Path(args.csv)
    if not csv_path.exists():
        print(f"Ошибка: файл {args.csv} не найден")
        print("Сначала запустите bash скрипт для генерации данных:")
        print("./benchmark_script.sh")
        sys.exit(1)
    
    print(f"Чтение данных из {args.csv}...")
    df = read_benchmark_data(args.csv)
    
    if df.empty or len(df) < 2:
        print("Недостаточно данных для построения графика")
        print("Убедитесь, что CSV файл содержит результаты тестов")
        sys.exit(1)
    
    if args.type in ['line', 'both']:
        plot_file = args.output
        if args.type == 'both':
            plot_file = 'benchmark_line_plot.png'
        
        create_benchmark_plot(df, plot_file, args.show)
    
    if args.type in ['bar', 'both']:
        bar_file = args.output
        if args.type == 'both':
            bar_file = 'benchmark_bar_plot.png'
        
        create_grouped_bar_chart(df, bar_file, args.show)
    
    print("\n" + "="*60)
    print("Сводная статистика:")
    print("="*60)
    
    for file_size in df['file_size'].unique():
        size_data = df[df['file_size'] == file_size]
        avg_rps = size_data['requests_per_second'].mean()
        max_rps = size_data['requests_per_second'].max()
        min_rps = size_data['requests_per_second'].min()
        
        print(f"\nРазмер файла: {file_size}")
        print(f"  Средний RPS: {avg_rps:.1f}")
        print(f"  Максимальный RPS: {max_rps:.1f}")
        print(f"  Минимальный RPS: {min_rps:.1f}")
        
        optimal_concurrency = size_data.loc[size_data['requests_per_second'].idxmax(), 'concurrency']
        print(f"  Оптимальный concurrency: {optimal_concurrency}")
    
    print("\n" + "="*60)

if __name__ == "__main__":
    main()