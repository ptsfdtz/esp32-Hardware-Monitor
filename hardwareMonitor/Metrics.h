#pragma once

struct Metric
{
  const char *label;
  float currentValue;
  float targetValue;
  int maxValue;
};

struct TemperatureMetric
{
  const char *label;
  int value;
};

constexpr int UNKNOWN_TEMPERATURE = -1;

extern Metric metrics[3];
extern TemperatureMetric temperatures[2];

void updateAnimation(float dt);
