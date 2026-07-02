#include "DisplayDashboard.h"

#include "Config.h"
#include "Metrics.h"
#include <Adafruit_GFX.h>
#include <Adafruit_SSD1306.h>
#include <Arduino.h>
#include <Wire.h>
#include <math.h>

static Adafruit_SSD1306 display(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, OLED_RESET);
static unsigned long lastTemperatureFrame = 0;

static bool selectDisplayChannel(int channel);
static bool beginDisplayChannel(int channel);
static void drawUsageScreen();
static void drawTemperatureScreen(TemperatureMetric metric);
static bool shouldDrawTemperatureScreens();
static void drawMetricRow(int rowY, Metric metric);
static void drawPremiumBar(int x, int y, int w, int h, float percent);
static void drawSegmentText(int x, int y, const char *text);
static void drawSegmentLetter(int x, int y, char c);
static void drawValueBlock(int y, int value);
static void drawTemperatureValue(int value);
static void drawNoTemperatureValue();
static void drawRoundedDigit(int x, int y, int num, int scale);
static void drawRoundedLetterC(int x, int y, int scale);
static void drawRoundedDegreeC(int x, int y, int scale);
static void drawRoundedDash(int x, int y, int scale);
static void drawRoundedSegment(int x, int y, int segment, int scale);
static void drawSevenDigit(int x, int y, int num);
static void segA(int x, int y);
static void segB(int x, int y);
static void segC(int x, int y);
static void segD(int x, int y);
static void segE(int x, int y);
static void segF(int x, int y);
static void segG(int x, int y);
static void drawPercentIcon(int x, int y);

static const bool DIGIT_SEGMENTS[10][7] = {
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

bool beginDisplay()
{
  Wire.begin(OLED_SDA, OLED_SCL);
  Wire.setClock(400000);

  if (!beginDisplayChannel(OLED_USAGE_CHANNEL))
  {
    return false;
  }

  if (!beginDisplayChannel(OLED_CPU_TEMP_CHANNEL))
  {
    return false;
  }

  if (!beginDisplayChannel(OLED_GPU_TEMP_CHANNEL))
  {
    return false;
  }

  return true;
}

void drawDashboard()
{
  if (selectDisplayChannel(OLED_USAGE_CHANNEL))
  {
    drawUsageScreen();
    display.display();
  }

  if (shouldDrawTemperatureScreens())
  {
    if (selectDisplayChannel(OLED_CPU_TEMP_CHANNEL))
    {
      drawTemperatureScreen(temperatures[0]);
      display.display();
    }

    if (selectDisplayChannel(OLED_GPU_TEMP_CHANNEL))
    {
      drawTemperatureScreen(temperatures[1]);
      display.display();
    }
  }
}

static bool selectDisplayChannel(int channel)
{
  if (channel < 0 || channel > 7)
  {
    return false;
  }

  Wire.beginTransmission(PCA9848A_ADDR);
  Wire.write(1 << channel);
  return Wire.endTransmission() == 0;
}

static bool beginDisplayChannel(int channel)
{
  if (!selectDisplayChannel(channel))
  {
    return false;
  }

  if (!display.begin(SSD1306_SWITCHCAPVCC, OLED_ADDR, false, false))
  {
    return false;
  }

  display.clearDisplay();
  display.display();
  return true;
}

static void drawUsageScreen()
{
  display.clearDisplay();
  drawMetricRow(2, metrics[0]);
  drawMetricRow(23, metrics[1]);
  drawMetricRow(44, metrics[2]);
}

static void drawTemperatureScreen(TemperatureMetric metric)
{
  display.clearDisplay();
  display.setTextColor(SSD1306_WHITE);

  if (metric.value == UNKNOWN_TEMPERATURE)
  {
    drawNoTemperatureValue();
  }
  else
  {
    drawTemperatureValue(metric.value);
  }
}

static bool shouldDrawTemperatureScreens()
{
  unsigned long now = millis();

  if (lastTemperatureFrame == 0 || now - lastTemperatureFrame >= TEMPERATURE_FRAME_INTERVAL)
  {
    lastTemperatureFrame = now;
    return true;
  }

  return false;
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

static void drawTemperatureValue(int value)
{
  value = constrain(value, 0, 199);

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

  const int scale = 3;
  const int digitW = 7 * scale;
  const int digitH = 14 * scale;
  const int gap = 5;
  const int unitGap = 7;
  const int unitW = 10 * scale;
  const int totalW = count * digitW + (count - 1) * gap + unitGap + unitW;
  int x = (SCREEN_WIDTH - totalW) / 2;
  int y = (SCREEN_HEIGHT - digitH) / 2;

  if (x < 0)
  {
    x = 0;
  }

  for (int i = 0; i < count; i++)
  {
    drawRoundedDigit(x + i * (digitW + gap), y, digits[i], scale);
  }

  drawRoundedDegreeC(x + count * digitW + (count - 1) * gap + unitGap, y, scale);
}

static void drawNoTemperatureValue()
{
  const int scale = 3;
  const int digitW = 7 * scale;
  const int digitH = 14 * scale;
  const int gap = 5;
  int x = (SCREEN_WIDTH - digitW * 2 - gap) / 2;
  int y = (SCREEN_HEIGHT - digitH) / 2;

  drawRoundedDash(x, y, scale);
  drawRoundedDash(x + digitW + gap, y, scale);
}

static void drawRoundedDigit(int x, int y, int num, int scale)
{
  num = constrain(num, 0, 9);

  for (int segment = 0; segment < 7; segment++)
  {
    if (DIGIT_SEGMENTS[num][segment])
    {
      drawRoundedSegment(x, y, segment, scale);
    }
  }
}

static void drawRoundedLetterC(int x, int y, int scale)
{
  drawRoundedSegment(x, y, 0, scale);
  drawRoundedSegment(x, y, 3, scale);
  drawRoundedSegment(x, y, 4, scale);
  drawRoundedSegment(x, y, 5, scale);
}

static void drawRoundedDegreeC(int x, int y, int scale)
{
  int degreeRadius = scale;
  display.drawCircle(x + degreeRadius, y + degreeRadius, degreeRadius, SSD1306_WHITE);
  display.drawCircle(x + degreeRadius, y + degreeRadius, max(1, degreeRadius - 1), SSD1306_WHITE);
  drawRoundedLetterC(x + 3 * scale, y, scale);
}

static void drawRoundedDash(int x, int y, int scale)
{
  drawRoundedSegment(x, y, 6, scale);
}

static void drawRoundedSegment(int x, int y, int segment, int scale)
{
  int thickness = 2 * scale;
  int radius = scale;

  switch (segment)
  {
  case 0:
    display.fillRoundRect(x + scale, y, 5 * scale, thickness, radius, SSD1306_WHITE);
    break;
  case 1:
    display.fillRoundRect(x + 5 * scale, y + scale, thickness, 5 * scale, radius, SSD1306_WHITE);
    break;
  case 2:
    display.fillRoundRect(x + 5 * scale, y + 7 * scale, thickness, 5 * scale, radius, SSD1306_WHITE);
    break;
  case 3:
    display.fillRoundRect(x + scale, y + 12 * scale, 5 * scale, thickness, radius, SSD1306_WHITE);
    break;
  case 4:
    display.fillRoundRect(x, y + 7 * scale, thickness, 5 * scale, radius, SSD1306_WHITE);
    break;
  case 5:
    display.fillRoundRect(x, y + scale, thickness, 5 * scale, radius, SSD1306_WHITE);
    break;
  case 6:
    display.fillRoundRect(x + scale, y + 6 * scale, 5 * scale, thickness, radius, SSD1306_WHITE);
    break;
  }
}

static void drawSevenDigit(int x, int y, int num)
{
  num = constrain(num, 0, 9);

  if (DIGIT_SEGMENTS[num][0])
    segA(x, y);
  if (DIGIT_SEGMENTS[num][1])
    segB(x, y);
  if (DIGIT_SEGMENTS[num][2])
    segC(x, y);
  if (DIGIT_SEGMENTS[num][3])
    segD(x, y);
  if (DIGIT_SEGMENTS[num][4])
    segE(x, y);
  if (DIGIT_SEGMENTS[num][5])
    segF(x, y);
  if (DIGIT_SEGMENTS[num][6])
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
