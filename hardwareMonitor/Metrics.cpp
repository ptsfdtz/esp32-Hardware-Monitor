#include "Metrics.h"

#include "Config.h"
#include <math.h>

Metric metrics[3] = {
    {"CPU", 0, 45, 100},
    {"GPU", 0, 52, 100},
    {"RAM", 0, 68, 100}};

void updateAnimation(float dt)
{
  float ease = 1.0f - expf(-ANIM_SPEED * dt);

  for (int i = 0; i < 3; i++)
  {
    metrics[i].currentValue += (metrics[i].targetValue - metrics[i].currentValue) * ease;
  }
}
