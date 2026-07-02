#pragma once

#include <Arduino.h>

constexpr int OLED_SDA = 4;
constexpr int OLED_SCL = 5;

constexpr int SCREEN_WIDTH = 128;
constexpr int SCREEN_HEIGHT = 64;
constexpr int OLED_RESET = -1;
constexpr int OLED_ADDR = 0x3C;
constexpr int PCA9848A_ADDR = 0x70;
constexpr int OLED_USAGE_CHANNEL = 0;
constexpr int OLED_CPU_TEMP_CHANNEL = 1;
constexpr int OLED_GPU_TEMP_CHANNEL = 2;

constexpr int FRAME_INTERVAL = 18;
constexpr int TEMPERATURE_FRAME_INTERVAL = 1000;
constexpr float ANIM_SPEED = 6.5f;

constexpr int LABEL_X = 0;
constexpr int BAR_X = 28;
constexpr int BAR_W = 63;
constexpr int BAR_H = 11;
