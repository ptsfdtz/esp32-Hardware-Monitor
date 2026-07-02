#pragma once

struct Metric
{
  const char *label;
  float currentValue;
  float targetValue;
  int maxValue;
};

extern Metric metrics[3];

void updateAnimation(float dt);
