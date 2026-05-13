COQC ?= coqc
COQDEP ?= coqdep
VFILES := concat_injective.v lyapunov_decrease.v \
          cascade/cascade_health_bounded.v \
          cascade/cascade_collision_resistance.v \
          cascade/cascade_determinism.v
VOFILES := $(VFILES:.v=.vo)

all: .depend $(VOFILES)
.depend: $(VFILES)
	$(COQDEP) -Q . QASH $(VFILES) > .depend
-include .depend
%.vo: %.v
	$(COQC) -Q . QASH $<
clean:
	rm -f *.vo *.vos *.vok *.glob .depend
.PHONY: all clean
