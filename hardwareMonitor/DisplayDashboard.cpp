#include "DisplayDashboard.h"

#include "Config.h"
#include "Metrics.h"
#include <Adafruit_GFX.h>
#include <Adafruit_SSD1306.h>
#include <Arduino.h>
#include <Wire.h>
#include <math.h>

static Adafruit_SSD1306 display(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, OLED_RESET);

static void drawMetricRow(int rowY, Metric metric);
static void drawPremiumBar(int x, int y, int w, int h, float percent);
static void drawSegmentText(int x, int y, const char *text);
static void drawSegmentLetter(int x, int y, char c);
static void drawValueBlock(int y, int value);
static void drawSevenDigit(int x, int y, int num);
static void segA(int x, int y);
static void segB(int x, int y);
static void segC(int x, int y);
static void segD(int x, int y);
static void segE(int x, int y);
static void segF(int x, int y);
static void segG(int x, int y);
static void drawPercentIcon(int x, int y);

bool beginDisplay()
{
  Wire.begin(OLED_SDA, OLED_SCL);
  Wire.setClock(400000);

  if (!display.begin(SSD1306_SWITCHCAPVCC, OLED_ADDR))
  {
    return false;
  }

  display.clearDisplay();
  display.display();
  return true;
}

void drawDashboard()
{
  display.clearDisplay();

  drawMetricRow(2, metrics[0]);
  drawMetricRow(23, metrics[1]);
  drawMetricRow(44, metrics[2]);

  display.display();
}

static void drawMetricRow(int rowY, Metric metric)
{
  int value = roundf(metric.currentValue);

  float barValue = constrain(metric.currentValue, 0.0f, (float)metric.maxValue);
  float percent = barValue / metric.maxValue;

  const int textH = 14;
  int textY = rowY + 1;
  int barY = textY + (textH - BAR_H + 1) / 2;

  drawSegmentText(LABEL_X, textY, metric.label);
  drawPremiumBar(BAR_X, barY, BAR_W, BAR_H, percent);
  drawValueBlock(textY, value);
}

static void drawPremiumBar(int x, int y, int w, int h, float percent)
{
  percent = constrain(percent, 0.0f, 1.0f);

  display.drawRoundRect(x, y, w, h, h / 2, SSD1306_WHITE);

  int innerX = x + 2;
  int innerY = y + 2;
  int innerW = w - 4;
  int innerH = h - 4;

  int fillW = roundf(innerW * percent);

  if (fillW <= 0)
  {
    return;
  }

  if (fillW < innerH)
  {
    display.fillCircle(
        innerX + fillW / 2,
        innerY + innerH / 2,
        max(1, fillW / 2),
        SSD1306_WHITE);
  }
  else
  {
    display.fillRoundRect(
        innerX,
        innerY,
        fillW,
        innerH,
        innerH / 2,
        SSD1306_WHITE);
  }

  if (fillW > innerH)
  {
    display.fillCircle(
        innerX + fillW - 1,
        innerY + innerH / 2,
        innerH / 2,
        SSD1306_WHITE);
  }

  if (fillW > 18)
  {
    int shimmer = (millis() / 70) % 18;
    int sx = innerX + shimmer;

    while (sx < innerX + fillW - 2)
    {
      display.drawPixel(sx, innerY + 1, SSD1306_BLACK);
      display.drawPixel(sx + 1, innerY + 2, SSD1306_BLACK);
      sx += 18;
    }
  }
}

static void drawSegmentText(int x, int y, const char *text)
{
  int cursorX = x;

  for (int i = 0; text[i] != '\0'; i++)
  {
    drawSegmentLetter(cursorX, y, text[i]);
    cursorX += 9;
  }
}

static void drawSegmentLetter(int x, int y, char c)
{
  switch (c)
  {
  case 'C':
    segA(x, y);
    segD(x, y);
    segE(x, y);
    segF(x, y);
    break;

  case 'P':
    segA(x, y);
    segB(x, y);
    segE(x, y);
    segF(x, y);
    segG(x, y);
    break;

  case 'U':
    segB(x, y);
    segC(x, y);
    segD(x, y);
    segE(x, y);
    segF(x, y);
    break;

  case 'G':
    segA(x, y);
    segC(x, y);
    segD(x, y);
    segE(x, y);
    segF(x, y);
    segG(x, y);
    break;

  case 'R':
    segA(x, y);
    segB(x, y);
    segE(x, y);
    segF(x, y);
    segG(x, y);
    display.drawLine(x + 3, y + 8, x + 6, y + 13, SSD1306_WHITE);
    display.drawLine(x + 4, y + 8, x + 7, y + 13, SSD1306_WHITE);
    break;

  case 'A':
    segA(x, y);
    segB(x, y);
    segC(x, y);
    segE(x, y);
    segF(x, y);
    segG(x, y);
    break;

  case 'M':
    segB(x, y);
    segC(x, y);
    segE(x, y);
    segF(x, y);
    display.drawLine(x + 2, y + 1, x + 3, y + 5, SSD1306_WHITE);
    display.drawLine(x + 5, y + 1, x + 4, y + 5, SSD1306_WHITE);
    display.drawLine(x + 3, y + 5, x + 4, y + 5, SSD1306_WHITE);
    break;
  }
}

static void drawValueBlock(int y, int value)
{
  value = constrain(value, 0, 999);

  int digits[3];
  int count = 0;

  if (value >= 100)
  {
    digits[0] = value / 100;
    digits[1] = (value / 10) % 10;
    digits[2] = value % 10;
    count = 3;
  }
  else if (value >= 10)
  {
    digits[0] = value / 10;
    digits[1] = value % 10;
    count = 2;
  }
  else
  {
    digits[0] = 0;
    digits[1] = value;
    count = 2;
  }

  int digitW = 7;
  int gap = 2;
  int totalW = count * digitW + (count - 1) * gap;

  int suffixX = 120;
  int numberRight = suffixX - 3;
  int startX = numberRight - totalW + 1;

  for (int i = 0; i < count; i++)
  {
    drawSevenDigit(startX + i * (digitW + gap), y, digits[i]);
  }

  drawPercentIcon(suffixX, y + 2);
}

static void drawSevenDigit(int x, int y, int num)
{
  bool seg[10][7] = {
      {1, 1, 1, 1, 1, 1, 0},
      {0, 1, 1, 0, 0, 0, 0},
      {1, 1, 0, 1, 1, 0, 1},
      {1, 1, 1, 1, 0, 0, 1},
      {0, 1, 1, 0, 0, 1, 1},
      {1, 0, 1, 1, 0, 1, 1},
      {1, 0, 1, 1, 1, 1, 1},
      {1, 1, 1, 0, 0, 0, 0},
      {1, 1, 1, 1, 1, 1, 1},
      {1, 1, 1, 1, 0, 1, 1}};

  num = constrain(num, 0, 9);

  if (seg[num][0])
    segA(x, y);
  if (seg[num][1])
    segB(x, y);
  if (seg[num][2])
    segC(x, y);
  if (seg[num][3])
    segD(x, y);
  if (seg[num][4])
    segE(x, y);
  if (seg[num][5])
    segF(x, y);
  if (seg[num][6])
    segG(x, y);
}

static void segA(int x, int y)
{
  display.fillRect(x + 1, y, 5, 2, SSD1306_WHITE);
}

static void segB(int x, int y)
{
  display.fillRect(x + 5, y + 1, 2, 5, SSD1306_WHITE);
}

static void segC(int x, int y)
{
  display.fillRect(x + 5, y + 7, 2, 5, SSD1306_WHITE);
}

static void segD(int x, int y)
{
  display.fillRect(x + 1, y + 12, 5, 2, SSD1306_WHITE);
}

static void segE(int x, int y)
{
  display.fillRect(x, y + 7, 2, 5, SSD1306_WHITE);
}

static void segF(int x, int y)
{
  display.fillRect(x, y + 1, 2, 5, SSD1306_WHITE);
}

static void segG(int x, int y)
{
  display.fillRect(x + 1, y + 6, 5, 2, SSD1306_WHITE);
}

static void drawPercentIcon(int x, int y)
{
  display.drawCircle(x + 1, y + 1, 1, SSD1306_WHITE);
  display.drawCircle(x + 6, y + 8, 1, SSD1306_WHITE);
  display.drawLine(x + 1, y + 9, x + 7, y + 1, SSD1306_WHITE);
}
