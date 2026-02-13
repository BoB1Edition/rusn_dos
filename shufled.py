#!/usr/bin/env python

import random

# Имя файла
file_path = 'opcodes.txt'

# Читаем байты из файла
with open(file_path, 'r') as f:
    bytes_list = [line.strip() for line in f if line.strip()]

# Перемешиваем список байт
random.shuffle(bytes_list)

# Выводим результат или сохраняем
# Для вывода:
print('\n'.join(bytes_list))

# Для сохранения в новый файл:
with open('shuffled_opcodes.txt', 'w') as f:
    f.write('\n'.join(bytes_list))